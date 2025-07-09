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
    ListDirRequest, ListDirResponse, Message, MessageType, UploadFileRequest,
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

/// Helper function to send a command and await a response from the client
async fn send_command_and_await_response<T: Serialize>(
    state: &AppState,
    client_id: &str,
    command_type: MessageType,
    request_data: &T, // Changed to generic serializable type
) -> Result<Message, FileOperationError> {
    let command_request = CommandRequest {
        client_id: client_id.to_string(),
        command: command_type.to_string(), // Use MessageType as command string
        args: vec![serde_json::to_string(request_data)?], // Serialize request_data into args
    };

    let message = Message::new(
        MessageType::ExecuteCommand,
        serde_json::to_vec(&command_request)?,
    );
    let message_id = message.id.clone();

    state
        .client_manager
        .add_command(client_id, command_request)
        .await;

    // Poll for the response with a timeout
    let response_message =
        tokio::time::timeout(Duration::from_secs(CLIENT_RESPONSE_TIMEOUT_SEC), async {
            loop {
                if let Some(response) = state
                    .client_manager
                    .get_file_operation_response(client_id, &message_id)
                    .await
                {
                    return Ok::<Message, FileOperationError>(response);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| {
            error!(
                "Client response timed out for client_id: {}, message_id: {}",
                client_id, message_id
            );
            FileOperationError::Other("Client response timed out".to_string())
        })??; // Double ?? for Result<Result<T, E>, E>

    Ok(response_message)
}

/// API handler to list directory contents on a client.
pub async fn list_directory_handler(
    State(state): State<AppState>,
    Json(payload): Json<ListDirectoryRequest>,
) -> Result<Json<ServerFileOperationResponse>, FileOperationError> {
    let client_id = payload.client_id;
    let path_str = payload.path.unwrap_or_else(|| "/".to_string()); // Default to root for client
    let recursive = payload.recursive.unwrap_or(false);

    info!(
        "Received request to list directory on client {}: {:?}, recursive: {}",
        client_id, path_str, recursive
    );

    let list_req = ListDirRequest {
        path: path_str,
        recursive,
    };

    let response_message = send_command_and_await_response(
        &state,
        &client_id,
        MessageType::ListDir,
        &list_req, // Pass the struct directly
    )
    .await?;

    let list_res: ListDirResponse = serde_json::from_slice(&response_message.payload)?;

    if list_res.success {
        Ok(Json(ServerFileOperationResponse {
            success: true,
            message: list_res.message,
            data: Some(serde_json::to_value(list_res.entries)?),
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
    Path(file_path): Path<String>,      // Extract file_path
    Query(query): Query<ClientIdQuery>, // Extract client_id from query
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
        MessageType::DownloadFile, // This will be DOWNLOAD_FILE_INIT on client
        &download_init_req,        // Pass the struct directly
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
        (client_id.clone(), file_id.clone(), state.clone()), // Initial state for the unfold stream
        move |(client_id, file_id, state)| async move {
            let response_message = match send_command_and_await_response(
                &state,
                &client_id,
                MessageType::DownloadFile, // This will be DOWNLOAD_FILE_CHUNK on client
                &file_id,                  // Pass the file_id string directly
            )
            .await
            {
                Ok(msg) => msg,
                Err(e) => return Some((Err(e), (client_id, file_id, state))),
            };

            let file_chunk: FileChunk = match serde_json::from_slice(&response_message.payload) {
                Ok(chunk) => chunk,
                Err(e) => {
                    error!("Failed to deserialize FileChunk: {}", e);
                    return Some((
                        Err(FileOperationError::SerializationError(e.to_string())),
                        (client_id, file_id, state),
                    ));
                }
            };

            if file_chunk.is_last {
                None
            } else {
                Some((
                    Ok(Bytes::from(file_chunk.chunk)),
                    (client_id, file_id, state),
                ))
            }
        },
    )
    .map(|res| res.map_err(|e| axum::Error::new(e))); // Convert FileOperationError to axum::Error

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
        MessageType::UploadFile, // This will be UPLOAD_FILE_INIT on client
        &upload_init_req,        // Pass the struct directly
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
    while let Some(chunk_result) = data_stream.next().await {
        // Iterate over the stream
        let chunk_bytes = chunk_result.map_err(|e| FileOperationError::IoError(e.to_string()))?;

        let file_chunk = FileChunk {
            file_id: file_id.clone(),
            chunk: chunk_bytes.to_vec(),
            is_last: false, // Will be set to true for the last chunk
            offset,
        };

        // Send chunk command to client
        let response_message = send_command_and_await_response(
            &state,
            &client_id,
            MessageType::UploadFile, // This will be UPLOAD_FILE_CHUNK on client
            &file_chunk,             // Pass the struct directly
        )
        .await?;

        let chunk_res: serde_json::Value = serde_json::from_slice(&response_message.payload)?;
        if !chunk_res["success"].as_bool().unwrap_or(false) {
            return Err(FileOperationError::Other(format!(
                "Client failed to upload chunk: {}",
                chunk_res["message"].as_str().unwrap_or("unknown error")
            )));
        }

        offset += chunk_bytes.len() as u64;
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
        MessageType::UploadFile, // This will be UPLOAD_FILE_CHUNK on client
        &final_chunk,            // Pass the struct directly
    )
    .await?;

    Ok(Json(ServerFileOperationResponse {
        success: true,
        message: "File uploaded successfully to client.".to_string(),
        data: None,
    }))
}
