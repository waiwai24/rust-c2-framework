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
use bytes::Bytes; // For bytes::Bytes in download stream
use common::message::{
    CommandRequest, DeletePathRequest, DeletePathResponse, DownloadFileRequest, FileChunk,
    FileOperationCommand, ListDirRequest, ListDirResponse, Message, MessageType, UploadFileRequest,
}; // Import new message types
use futures::stream::{self, StreamExt};
use log::{error, info}; // Import error for logging
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::time::Duration; // For polling with timeout
use uuid::Uuid; // Import Uuid // For stream::unfold and StreamExt::map

const CLIENT_RESPONSE_TIMEOUT_SEC: u64 = 60; // Timeout for client responses

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
        message_id: None, // Temporary, will be updated
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
        message_id: Some(message_id.clone()), // Include message_id for event-driven responses
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
        // Ensure we return the entries array in the expected format
        let entries_json = serde_json::to_value(&list_res.entries)?;
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

/// API handler to delete a file or directory on a client.
pub async fn delete_path_handler(
    State(state): State<AppState>,
    Json(payload): Json<DeletePathPayloadWithClient>,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    // Changed return type
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
        &delete_req, // Pass the struct directly
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

/// API handler for downloading a file from a client to the server.
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
            // For chunk requests, send file_id directly as the request data
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

            // Check if this is the last chunk or if download is complete
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

/// API handler for uploading a file from the server to a client.
pub async fn upload_file_handler(
    State(state): State<AppState>,
    Path(file_path): Path<String>,      // Extract file_path
    Query(query): Query<ClientIdQuery>, // Extract client_id from query
    body: axum::body::Body,             // Removed mut as it's not needed for into_data_stream
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = query.client_id;
    // Changed return type
    info!(
        "Received request to upload file {:?} to client {}",
        file_path, client_id
    );

    let file_id = Uuid::new_v4().to_string(); // Generate a unique ID for this upload session

    let upload_init_req = UploadFileRequest {
        path: file_path.clone(),
        file_id: file_id.clone(),
    };

    // Send initiation command to client
    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::UploadFileInit, // This will be UPLOAD_FILE_INIT on client
        &upload_init_req,            // Pass the struct directly
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
    let mut data_stream = body.into_data_stream(); // Get the data stream from the body
    let mut buffer = Vec::new();
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for combining small chunks

    while let Some(chunk_result) = data_stream.next().await {
        // Iterate over the stream
        let chunk_bytes = chunk_result.map_err(|e| FileOperationError::IoError(e.to_string()))?;

        // Accumulate data in buffer
        buffer.extend_from_slice(&chunk_bytes);

        // Send buffer when it's large enough or this is the last chunk
        if buffer.len() >= BUFFER_SIZE {
            let file_chunk = FileChunk {
                file_id: file_id.clone(),
                chunk: buffer.clone(),
                is_last: false,
                offset,
            };

            // Send chunk command to client using new command type
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

    // Send any remaining data in buffer
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

    // Send a final chunk with is_last = true to signal end of file
    let final_chunk = FileChunk {
        file_id: file_id.clone(),
        chunk: Vec::new(), // Empty chunk for signaling end
        is_last: true,
        offset,
    };
    send_command_and_await_response(
        &state,
        &client_id,
        MessageType::UploadFileChunk, // This will be UPLOAD_FILE_CHUNK on client
        &final_chunk,                 // Pass the struct directly
    )
    .await?;

    Ok(Json(ServerFileOperationResponse {
        success: true,
        message: "File uploaded successfully to client.".to_string(),
        data: None,
    }))
}
