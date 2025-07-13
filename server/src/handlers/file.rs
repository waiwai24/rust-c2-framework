use crate::{
    error::{FileOperationError, ServerFileOperationResponse},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use bytes::Bytes;
use common::message::{
    CommandRequest, DeletePathRequest, DeletePathResponse, DownloadChunkRequest,
    DownloadFileRequest, FileChunk, FileEntry, FileOperationCommand, ListDirRequest,
    ListDirResponse, Message, MessageType, UploadFileRequest,
};
use futures::stream::{self, StreamExt};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;
use tokio::time::Duration;
use tracing::{error, info, instrument};
use uuid::Uuid;

const CLIENT_RESPONSE_TIMEOUT_SEC: u64 = 15; // Reduced timeout for faster failure detection

// Static regex for permission parsing
static PERMISSION_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_permission_regex() -> &'static Regex {
    PERMISSION_REGEX.get_or_init(|| Regex::new(r"mode: 0o(\d+) \(([d\-rwxstST]+)\)").unwrap())
}

/// Represents a file entry with additional metadata
#[derive(Debug, Deserialize)]
pub struct ListDirectoryRequest {
    pub client_id: String,
    pub path: Option<String>,
    pub recursive: Option<bool>,
}

/// Represents a file entry with additional metadata
#[derive(Debug, Deserialize)]
pub struct DeletePathPayloadWithClient {
    pub client_id: String,
    pub path: String,
}

/// Represents a request to download a file
#[derive(Debug, Deserialize)]
pub struct ClientIdQuery {
    pub client_id: String,
}

/// Helper function to send a command and await a response from the client using event-driven mechanism
async fn send_command_and_await_response_event_driven<T: Serialize>(
    state: &AppState,
    client_id: &str,
    command_type: MessageType,
    request_data: &T,
) -> Result<Message, FileOperationError> {
    // First create the message to get the ID
    let temp_command_request = CommandRequest {
        client_id: client_id.to_string(),
        command: command_type.to_string(),
        args: vec![serde_json::to_string(request_data)?],
        message_id: None,
        shellcode: None, // Add shellcode field
    };

    let message = Message::new(
        MessageType::ExecuteCommand,
        serde_json::to_vec(&temp_command_request)?,
    );
    let message_id = message.id.clone();

    // Now create the final command request with the message_id
    let command_request = CommandRequest {
        client_id: client_id.to_string(),
        command: command_type.to_string(),
        args: vec![serde_json::to_string(request_data)?],
        message_id: Some(message_id.clone()),
        shellcode: None, // Add shellcode field
    };

    // Register for event-driven response
    let response_receiver = state.register_response_notifier(message_id.clone()).await;

    // Send command to client
    state
        .client_manager
        .add_command(client_id, command_request)
        .await;

    // Wait for event-driven response with timeout
    let response_message = tokio::time::timeout(
        Duration::from_secs(CLIENT_RESPONSE_TIMEOUT_SEC),
        response_receiver,
    )
    .await
    .map_err(|_| {
        error!(
            "Client response timed out for client_id: {}, message_id: {}",
            client_id, message_id
        );
        FileOperationError::Other("Client response timed out".to_string())
    })?
    .map_err(|_| {
        error!(
            "Response channel closed for client_id: {}, message_id: {}",
            client_id, message_id
        );
        FileOperationError::Other("Response channel closed".to_string())
    })?;

    Ok(response_message)
}

/// Alias for backward compatibility - now uses event-driven mechanism
async fn send_command_and_await_response<T: Serialize>(
    state: &AppState,
    client_id: &str,
    command_type: MessageType,
    request_data: &T,
) -> Result<Message, FileOperationError> {
    send_command_and_await_response_event_driven(state, client_id, command_type, request_data).await
}

/// Parse permissions from debug string format to clean Unix format
fn parse_permissions(debug_str: &str) -> Option<String> {
    let regex = get_permission_regex();

    if let Some(captures) = regex.captures(debug_str) {
        if let Some(perm_str) = captures.get(2) {
            return Some(perm_str.as_str().to_string());
        }
    }

    // Fallback for older formats
    if debug_str.contains("(") && debug_str.contains(")") {
        if let Some(start) = debug_str.find('(') {
            if let Some(end) = debug_str.find(')') {
                if start < end {
                    let perm_part = &debug_str[start + 1..end];
                    // Check if it matches Unix permission format
                    if perm_part.len() >= 9 && perm_part.chars().all(|c| "drwxstST-".contains(c)) {
                        return Some(perm_part.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Enhanced file entry with parsed permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub permissions: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

/// Convert FileEntry to EnhancedFileEntry with parsed permissions
impl From<FileEntry> for EnhancedFileEntry {
    fn from(entry: FileEntry) -> Self {
        // Parse the permissions string
        let parsed_permissions = entry
            .permissions
            .as_ref()
            .and_then(|p| parse_permissions(p))
            .or(entry.permissions);

        Self {
            name: entry.name,
            path: entry.path.to_string_lossy().to_string(),
            is_dir: entry.is_dir,
            size: entry.size,
            modified: entry.modified,
            permissions: parsed_permissions,
            owner: entry.owner.clone(),
            group: entry.group.clone(),
        }
    }
}

/// API handler to list directory contents on a client.
pub async fn list_directory_handler(
    State(state): State<AppState>,
    Json(payload): Json<ListDirectoryRequest>,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = payload.client_id;
    let path_str = payload.path.unwrap_or_else(|| "/".to_string());
    let recursive = payload.recursive.unwrap_or(false);

    info!(
        "Received request to list directory on client {}: {:?}, recursive: {}",
        client_id, path_str, recursive
    );

    // Log the list operation start
    state
        .audit_logger
        .log_file_operation(&client_id, "LIST", &path_str, None, "STARTED");

    let list_req = ListDirRequest {
        path: path_str.clone(),
        recursive,
    };

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::ExecuteCommand,
        &FileOperationCommand::ListDir(list_req),
    )
    .await?;

    info!(
        "Raw payload received for ListDir: {:?}",
        String::from_utf8_lossy(&response_message.payload)
    );

    let list_res: ListDirResponse =
        serde_json::from_slice(&response_message.payload).map_err(|e| {
            error!("Failed to deserialize ListDirResponse: {}", e);
            FileOperationError::SerializationError(e.to_string())
        })?;

    info!("Deserialized ListDirResponse: {:?}", list_res);

    if list_res.success {
        // Convert FileEntry to EnhancedFileEntry with parsed permissions
        let enhanced_entries: Vec<EnhancedFileEntry> = list_res
            .entries
            .into_iter()
            .map(EnhancedFileEntry::from)
            .collect();

        // Log successful list operation
        state.audit_logger.log_file_operation(
            &client_id,
            "LIST",
            &path_str,
            Some(enhanced_entries.len() as u64),
            "SUCCESS",
        );

        let entries_json = serde_json::to_value(&enhanced_entries)?;
        Ok(Json(ServerFileOperationResponse {
            success: true,
            message: list_res.message,
            data: Some(serde_json::json!({
                "entries": entries_json
            })),
        }))
    } else {
        // Log failed list operation
        state.audit_logger.log_file_operation(
            &client_id,
            "LIST",
            &path_str,
            None,
            &format!("FAILED: {}", list_res.message),
        );

        Err(FileOperationError::Other(list_res.message))
    }
}

/// API handler to list directory contents on a client.
pub async fn list_directory(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
    Json(req): Json<ListDirRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    info!(
        "Received list directory request for client {}: path={}, recursive={}",
        client_id, req.path, req.recursive
    );

    // Create command request
    let cmd = CommandRequest {
        client_id: client_id.clone(),
        command: "FileOperation".to_string(),
        args: vec![
            serde_json::to_string(&FileOperationCommand::ListDir(req.clone())).map_err(|e| {
                error!("Failed to serialize ListDirRequest: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to serialize request".to_string(),
                )
            })?,
        ],
        message_id: Some(Uuid::new_v4().to_string()),
        shellcode: None, // Add shellcode field
    };

    // Send command to client
    state
        .client_manager
        .add_command(&client_id, cmd.clone())
        .await;
    info!("Sent list directory command to client {}", client_id);

    // Wait for response using register_response_notifier instead
    let message_id = cmd.message_id.as_ref().unwrap();
    let response_receiver = state.register_response_notifier(message_id.clone()).await;

    let response_message =
        match tokio::time::timeout(Duration::from_secs(30), response_receiver).await {
            Ok(Ok(msg)) => {
                info!(
                    "Received response for list directory request: message_id={}",
                    msg.id
                );
                msg
            }
            Ok(Err(_)) => {
                error!(
                    "Response channel closed for list directory request from client {}",
                    client_id
                );
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Response channel closed".to_string(),
                ));
            }
            Err(_) => {
                error!(
                    "Timeout waiting for list directory response from client {}",
                    client_id
                );
                return Err((
                    StatusCode::REQUEST_TIMEOUT,
                    "Timeout waiting for response from client".to_string(),
                ));
            }
        };

    // Parse the response
    let payload_str = String::from_utf8(response_message.payload.clone()).map_err(|e| {
        error!("Failed to convert response payload to string: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid response format".to_string(),
        )
    })?;

    info!("Raw payload received for ListDir: {:?}", payload_str);

    let response: ListDirResponse = serde_json::from_str(&payload_str).map_err(|e| {
        error!("Failed to deserialize ListDirResponse: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse response".to_string(),
        )
    })?;

    info!("Deserialized ListDirResponse: {:?}", response);

    // Convert FileEntry to EnhancedFileEntry with parsed permissions
    let enhanced_entries: Vec<EnhancedFileEntry> = response
        .entries
        .into_iter()
        .map(EnhancedFileEntry::from)
        .collect();

    let enhanced_response = serde_json::json!({
        "entries": enhanced_entries,
        "success": response.success,
        "message": response.message
    });

    info!(
        "Successfully processed list directory request for client {}: found {} entries",
        client_id,
        enhanced_entries.len()
    );

    Ok(Json(enhanced_response))
}

/// API handler to delete a path on a client.
#[instrument(skip(state, payload), fields(client_id = %payload.client_id))]
pub async fn delete_path_handler(
    State(state): State<AppState>,
    Json(payload): Json<DeletePathPayloadWithClient>,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = payload.client_id;
    info!(
        client_id = %client_id,
        file_path = %payload.path,
        "Starting delete path operation"
    );

    // Log delete operation start
    state
        .audit_logger
        .log_file_operation(&client_id, "DELETE", &payload.path, None, "STARTED");

    let delete_req = DeletePathRequest {
        path: payload.path.clone(),
    };

    info!(
        client_id = %client_id,
        file_path = %payload.path,
        "Sending delete command to client"
    );

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::ExecuteCommand,
        &FileOperationCommand::DeletePath(delete_req),
    )
    .await
    .map_err(|e| {
        error!(
            client_id = %client_id,
            file_path = %payload.path,
            error = %e,
            "Failed to send delete command to client"
        );
        e
    })?;

    let delete_res: DeletePathResponse = serde_json::from_slice(&response_message.payload)
        .map_err(|e| {
            error!(
                client_id = %client_id,
                file_path = %payload.path,
                error = %e,
                "Failed to deserialize delete response"
            );
            FileOperationError::SerializationError(e.to_string())
        })?;

    if delete_res.success {
        info!(
            client_id = %client_id,
            file_path = %payload.path,
            operation_result = "success",
            "Delete path operation completed successfully"
        );

        // Log successful delete operation
        state
            .audit_logger
            .log_file_operation(&client_id, "DELETE", &payload.path, None, "SUCCESS");

        Ok(Json(ServerFileOperationResponse {
            success: true,
            message: delete_res.message,
            data: None,
        }))
    } else {
        error!(
            client_id = %client_id,
            file_path = %payload.path,
            operation_result = "failure",
            error_message = %delete_res.message,
            "Delete path operation failed"
        );

        // Log failed delete operation
        state.audit_logger.log_file_operation(
            &client_id,
            "DELETE",
            &payload.path,
            None,
            &format!("FAILED: {}", delete_res.message),
        );

        Err(FileOperationError::Other(delete_res.message))
    }
}

/// API handler to download a file from a client.
#[instrument(skip(state, query, file_path), fields(client_id = %query.client_id))]
pub async fn download_file_handler(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(query): Query<ClientIdQuery>,
) -> Result<Response, FileOperationError> {
    let client_id = query.client_id;
    info!(
        client_id = %client_id,
        file_path = %file_path,
        "Starting download file operation"
    );

    // Log download operation start
    state
        .audit_logger
        .log_file_operation(&client_id, "DOWNLOAD", &file_path, None, "STARTED");

    let download_init_req = DownloadFileRequest {
        path: file_path.clone(),
    };

    info!(
        client_id = %client_id,
        file_path = %file_path,
        "Sending download initialization command to client"
    );

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::ExecuteCommand,
        &FileOperationCommand::DownloadInit(download_init_req),
    )
    .await
    .map_err(|e| {
        error!(
            client_id = %client_id,
            file_path = %file_path,
            error = %e,
            "Failed to initialize download with client"
        );

        // Log failed download initialization
        state.audit_logger.log_file_operation(
            &client_id,
            "DOWNLOAD",
            &file_path,
            None,
            &format!("FAILED: {e}"),
        );

        e
    })?;

    let init_res: serde_json::Value =
        serde_json::from_slice(&response_message.payload).map_err(|e| {
            error!(
                client_id = %client_id,
                file_path = %file_path,
                error = %e,
                "Failed to deserialize download initialization response"
            );
            FileOperationError::SerializationError(e.to_string())
        })?;

    let file_id = init_res["file_id"]
        .as_str()
        .ok_or_else(|| {
            error!(
                client_id = %client_id,
                file_path = %file_path,
                "Missing file_id in download initialization response"
            );
            FileOperationError::Other("Missing file_id in download initiation response".to_string())
        })?
        .to_string();

    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        "Download initialization successful, starting chunk streaming"
    );

    let filename = PathBuf::from(&file_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let content_disposition = format!("attachment; filename=\"{filename}\"");

    // Create a stream to pull chunks from the client
    let chunk_count = 0u64;
    let total_bytes = 0u64;

    let stream = stream::unfold(
        (
            client_id.clone(),
            file_id.clone(),
            state.clone(),
            chunk_count,
            total_bytes,
            file_path.clone(),
        ),
        move |(client_id, file_id, state, mut chunk_count, mut total_bytes, file_path)| async move {
            chunk_count += 1;
            let chunk_request = DownloadChunkRequest {
                file_id: file_id.clone(),
            };

            let response_message = match send_command_and_await_response(
                &state,
                &client_id,
                MessageType::ExecuteCommand,
                &FileOperationCommand::DownloadChunk(chunk_request),
            )
            .await
            {
                Ok(msg) => msg,
                Err(e) => {
                    error!(
                        client_id = %client_id,
                        file_id = %file_id,
                        chunk_number = chunk_count,
                        error = %e,
                        "Failed to download file chunk"
                    );

                    // Log download chunk failure
                    state.audit_logger.log_file_operation(
                        &client_id,
                        "DOWNLOAD",
                        &file_path,
                        Some(total_bytes),
                        &format!("FAILED: chunk error - {e}"),
                    );

                    return Some((
                        Err(e),
                        (
                            client_id,
                            file_id,
                            state,
                            chunk_count,
                            total_bytes,
                            file_path,
                        ),
                    ));
                }
            };

            let chunk_res: serde_json::Value =
                match serde_json::from_slice(&response_message.payload) {
                    Ok(res) => res,
                    Err(e) => {
                        error!(
                            client_id = %client_id,
                            file_id = %file_id,
                            chunk_number = chunk_count,
                            error = %e,
                            "Failed to deserialize chunk response"
                        );

                        // Log deserialization error
                        state.audit_logger.log_file_operation(
                            &client_id,
                            "DOWNLOAD",
                            &file_path,
                            Some(total_bytes),
                            &format!("FAILED: deserialization error - {e}"),
                        );

                        return Some((
                            Err(FileOperationError::SerializationError(e.to_string())),
                            (
                                client_id,
                                file_id,
                                state,
                                chunk_count,
                                total_bytes,
                                file_path,
                            ),
                        ));
                    }
                };

            if chunk_res["is_last"].as_bool().unwrap_or(false)
                || chunk_res["message"]
                    .as_str()
                    .unwrap_or("")
                    .contains("complete")
            {
                info!(
                    client_id = %client_id,
                    file_id = %file_id,
                    chunk_count = chunk_count,
                    total_bytes = total_bytes,
                    operation_result = "success",
                    "Download completed successfully"
                );

                // Log successful download completion
                state.audit_logger.log_file_operation(
                    &client_id,
                    "DOWNLOAD",
                    &file_path,
                    Some(total_bytes),
                    "SUCCESS",
                );

                None
            } else if let Ok(file_chunk) = serde_json::from_value::<FileChunk>(chunk_res) {
                let chunk_size = file_chunk.chunk.len() as u64;
                total_bytes += chunk_size;

                if chunk_count % 100 == 0 {
                    info!(
                        client_id = %client_id,
                        file_id = %file_id,
                        chunk_number = chunk_count,
                        chunk_size = chunk_size,
                        total_bytes = total_bytes,
                        "Download progress update"
                    );
                }

                Some((
                    Ok(Bytes::from(file_chunk.chunk)),
                    (
                        client_id,
                        file_id,
                        state,
                        chunk_count,
                        total_bytes,
                        file_path,
                    ),
                ))
            } else {
                error!(
                    client_id = %client_id,
                    file_id = %file_id,
                    chunk_number = chunk_count,
                    "Invalid chunk response format"
                );

                // Log invalid chunk response
                state.audit_logger.log_file_operation(
                    &client_id,
                    "DOWNLOAD",
                    &file_path,
                    Some(total_bytes),
                    "FAILED: invalid chunk response format",
                );

                Some((
                    Err(FileOperationError::Other(
                        "Invalid chunk response".to_string(),
                    )),
                    (
                        client_id,
                        file_id,
                        state,
                        chunk_count,
                        total_bytes,
                        file_path,
                    ),
                ))
            }
        },
    )
    .map(|res| res.map_err(axum::Error::new));

    let body = axum::body::Body::from_stream(stream);

    info!(
        client_id = %client_id,
        file_path = %file_path,
        filename = %filename,
        "Sending download response with streaming body"
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Disposition", content_disposition)
        .body(body)
        .map_err(|e| {
            error!(
                client_id = %client_id,
                file_path = %file_path,
                error = %e,
                "Failed to build download response"
            );
            FileOperationError::Other(format!("Failed to build response: {e}"))
        })
}

/// API handler to upload a file to a client.
#[instrument(skip(state, query, body, file_path), fields(client_id = %query.client_id))]
pub async fn upload_file_handler(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(query): Query<ClientIdQuery>,
    body: axum::body::Body,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = query.client_id;
    info!(
        client_id = %client_id,
        file_path = %file_path,
        "Starting upload file operation"
    );

    // Log upload operation start
    state
        .audit_logger
        .log_file_operation(&client_id, "UPLOAD", &file_path, None, "STARTED");

    let file_id = Uuid::new_v4().to_string();

    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        "Generated file ID for upload operation"
    );

    let upload_init_req = UploadFileRequest {
        path: file_path.clone(),
        file_id: file_id.clone(),
    };

    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        "Sending upload initialization command to client"
    );

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::ExecuteCommand,
        &FileOperationCommand::UploadInit(upload_init_req),
    )
    .await
    .map_err(|e| {
        error!(
            client_id = %client_id,
            file_path = %file_path,
            file_id = %file_id,
            error = %e,
            "Failed to initialize upload with client"
        );

        // Log failed upload initialization
        state.audit_logger.log_file_operation(
            &client_id,
            "UPLOAD",
            &file_path,
            None,
            &format!("FAILED: {e}"),
        );

        e
    })?;

    let init_res: serde_json::Value =
        serde_json::from_slice(&response_message.payload).map_err(|e| {
            error!(
                client_id = %client_id,
                file_path = %file_path,
                file_id = %file_id,
                error = %e,
                "Failed to deserialize upload initialization response"
            );
            FileOperationError::SerializationError(e.to_string())
        })?;

    if !init_res["success"].as_bool().unwrap_or(false) {
        let error_msg = init_res["message"].as_str().unwrap_or("unknown error");
        error!(
            client_id = %client_id,
            file_path = %file_path,
            file_id = %file_id,
            error_message = %error_msg,
            "Client failed to initiate upload"
        );

        // Log failed upload initialization
        state.audit_logger.log_file_operation(
            &client_id,
            "UPLOAD",
            &file_path,
            None,
            &format!("FAILED: {error_msg}"),
        );

        return Err(FileOperationError::Other(format!(
            "Client failed to initiate upload: {error_msg}"
        )));
    }

    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        "Upload initialization successful, starting chunk upload"
    );

    let mut offset = 0;
    let mut chunk_number = 0u64;
    let mut data_stream = body.into_data_stream();
    let mut buffer = Vec::new();
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        buffer_size = BUFFER_SIZE,
        "Starting data stream processing"
    );

    while let Some(chunk_result) = data_stream.next().await {
        let chunk_bytes = chunk_result.map_err(|e| {
            error!(
                client_id = %client_id,
                file_path = %file_path,
                file_id = %file_id,
                error = %e,
                "Failed to read chunk from data stream"
            );

            // Log stream read error
            state.audit_logger.log_file_operation(
                &client_id,
                "UPLOAD",
                &file_path,
                Some(offset),
                &format!("FAILED: stream read error - {e}"),
            );

            FileOperationError::IoError(e.to_string())
        })?;

        buffer.extend_from_slice(&chunk_bytes);

        if buffer.len() >= BUFFER_SIZE {
            chunk_number += 1;
            let chunk_size = buffer.len();

            let file_chunk = FileChunk {
                file_id: file_id.clone(),
                chunk: buffer.clone(),
                is_last: false,
                offset,
            };

            info!(
                client_id = %client_id,
                file_path = %file_path,
                file_id = %file_id,
                chunk_number = chunk_number,
                chunk_size = chunk_size,
                offset = offset,
                "Sending chunk to client"
            );

            let response_message = send_command_and_await_response(
                &state,
                &client_id,
                MessageType::ExecuteCommand,
                &FileOperationCommand::UploadChunk(file_chunk),
            )
            .await
            .map_err(|e| {
                error!(
                    client_id = %client_id,
                    file_path = %file_path,
                    file_id = %file_id,
                    chunk_number = chunk_number,
                    error = %e,
                    "Failed to send chunk to client"
                );

                // Log chunk send error
                state.audit_logger.log_file_operation(
                    &client_id,
                    "UPLOAD",
                    &file_path,
                    Some(offset),
                    &format!("FAILED: chunk send error - {e}"),
                );

                e
            })?;

            let chunk_res: serde_json::Value = serde_json::from_slice(&response_message.payload)
                .map_err(|e| {
                    error!(
                        client_id = %client_id,
                        file_path = %file_path,
                        file_id = %file_id,
                        chunk_number = chunk_number,
                        error = %e,
                        "Failed to deserialize chunk response"
                    );

                    // Log chunk response deserialization error
                    state.audit_logger.log_file_operation(
                        &client_id,
                        "UPLOAD",
                        &file_path,
                        Some(offset),
                        &format!("FAILED: chunk response deserialization error - {e}"),
                    );

                    FileOperationError::SerializationError(e.to_string())
                })?;

            if !chunk_res["success"].as_bool().unwrap_or(false) {
                let error_msg = chunk_res["message"].as_str().unwrap_or("unknown error");
                error!(
                    client_id = %client_id,
                    file_path = %file_path,
                    file_id = %file_id,
                    chunk_number = chunk_number,
                    error_message = %error_msg,
                    "Client failed to upload chunk"
                );

                // Log chunk upload failure
                state.audit_logger.log_file_operation(
                    &client_id,
                    "UPLOAD",
                    &file_path,
                    Some(offset),
                    &format!("FAILED: client chunk upload error - {error_msg}"),
                );

                return Err(FileOperationError::Other(format!(
                    "Client failed to upload chunk: {error_msg}"
                )));
            }

            offset += buffer.len() as u64;
            buffer.clear();

            if chunk_number % 10 == 0 {
                info!(
                    client_id = %client_id,
                    file_path = %file_path,
                    file_id = %file_id,
                    chunk_number = chunk_number,
                    total_bytes = offset,
                    "Upload progress update"
                );
            }
        }
    }

    if !buffer.is_empty() {
        chunk_number += 1;
        let chunk_len = buffer.len();

        let file_chunk = FileChunk {
            file_id: file_id.clone(),
            chunk: buffer,
            is_last: false,
            offset,
        };

        info!(
            client_id = %client_id,
            file_path = %file_path,
            file_id = %file_id,
            chunk_number = chunk_number,
            chunk_size = chunk_len,
            offset = offset,
            "Sending final data chunk to client"
        );

        let response_message = send_command_and_await_response(
            &state,
            &client_id,
            MessageType::ExecuteCommand,
            &FileOperationCommand::UploadChunk(file_chunk),
        )
        .await
        .map_err(|e| {
            error!(
                client_id = %client_id,
                file_path = %file_path,
                file_id = %file_id,
                chunk_number = chunk_number,
                error = %e,
                "Failed to send final data chunk to client"
            );

            // Log final chunk send error
            state.audit_logger.log_file_operation(
                &client_id,
                "UPLOAD",
                &file_path,
                Some(offset),
                &format!("FAILED: final chunk send error - {e}"),
            );

            e
        })?;

        let chunk_res: serde_json::Value = serde_json::from_slice(&response_message.payload)
            .map_err(|e| {
                error!(
                    client_id = %client_id,
                    file_path = %file_path,
                    file_id = %file_id,
                    chunk_number = chunk_number,
                    error = %e,
                    "Failed to deserialize final chunk response"
                );

                // Log final chunk response deserialization error
                state.audit_logger.log_file_operation(
                    &client_id,
                    "UPLOAD",
                    &file_path,
                    Some(offset),
                    &format!("FAILED: final chunk response deserialization error - {e}"),
                );

                FileOperationError::SerializationError(e.to_string())
            })?;

        if !chunk_res["success"].as_bool().unwrap_or(false) {
            let error_msg = chunk_res["message"].as_str().unwrap_or("unknown error");
            error!(
                client_id = %client_id,
                file_path = %file_path,
                file_id = %file_id,
                chunk_number = chunk_number,
                error_message = %error_msg,
                "Client failed to upload final chunk"
            );

            // Log final chunk upload failure
            state.audit_logger.log_file_operation(
                &client_id,
                "UPLOAD",
                &file_path,
                Some(offset),
                &format!("FAILED: client final chunk upload error - {error_msg}"),
            );

            return Err(FileOperationError::Other(format!(
                "Client failed to upload chunk: {error_msg}"
            )));
        }

        offset += chunk_len as u64;
    }

    // Send final chunk
    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        total_bytes = offset,
        "Sending completion marker to client"
    );

    let final_chunk = FileChunk {
        file_id: file_id.clone(),
        chunk: Vec::new(),
        is_last: true,
        offset,
    };

    send_command_and_await_response(
        &state,
        &client_id,
        MessageType::ExecuteCommand,
        &FileOperationCommand::UploadChunk(final_chunk),
    )
    .await
    .map_err(|e| {
        error!(
            client_id = %client_id,
            file_path = %file_path,
            file_id = %file_id,
            error = %e,
            "Failed to send completion marker to client"
        );

        // Log completion marker send error
        state.audit_logger.log_file_operation(
            &client_id,
            "UPLOAD",
            &file_path,
            Some(offset),
            &format!("FAILED: completion marker send error - {e}"),
        );

        e
    })?;

    info!(
        client_id = %client_id,
        file_path = %file_path,
        file_id = %file_id,
        total_bytes = offset,
        chunk_count = chunk_number,
        operation_result = "success",
        "Upload operation completed successfully"
    );

    // Log successful upload completion
    state.audit_logger.log_file_operation(
        &client_id,
        "UPLOAD",
        &file_path,
        Some(offset),
        "SUCCESS",
    );

    Ok(Json(ServerFileOperationResponse {
        success: true,
        message: "File uploaded successfully to client.".to_string(),
        data: None,
    }))
}
