use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use common::message::CommandResponse;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// Display client information for rendering in templates
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub clients: Vec<DisplayClientInfo>,
    pub online_clients_count: usize,
    pub os_types_count: usize,
    pub refresh_interval: u64,
}

/// Client template for rendering client details
#[derive(Template)]
#[template(path = "client.html")]
pub struct ClientTemplate {
    pub client: DisplayClientInfo,
    pub command_results: Vec<CommandResponse>,
}

/// Error template for rendering error pages
#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub error_code: u16,
    pub error_message: String,
    pub error_detail: Option<String>,
}

/// Display client information for rendering in templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayClientInfo {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub ip: String,
    pub country_info: Option<String>,
    pub cpu_brand: String,
    pub cpu_frequency: u64,
    pub cpu_cores: usize,
    pub memory: u64,
    pub total_disk_space: u64,
    pub available_disk_space: u64,
    pub total_disk_space_gb: String,
    pub available_disk_space_gb: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub is_online: bool,
}

/// Main dashboard page
pub async fn index(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    info!("Dashboard accessed");

    let clients = match state.client_manager.get_clients().await {
        clients => clients,
    };

    let current_timestamp = chrono::Utc::now().timestamp();

    let display_clients: Vec<DisplayClientInfo> = clients
        .into_iter()
        .map(|c| {
            let is_online =
                (current_timestamp - c.last_seen.timestamp()) < state.config.client_timeout as i64;
            DisplayClientInfo {
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

    let template = IndexTemplate {
        clients: display_clients,
        online_clients_count,
        os_types_count,
        refresh_interval: state.config.web.refresh_interval,
    };

    match tokio::task::spawn_blocking(move || template.render()).await {
        Ok(Ok(html)) => {
            info!("Dashboard rendered successfully");
            Ok(Html(html))
        }
        Ok(Err(e)) => {
            error!("Template rendering failed for dashboard: {:?}", e);
            render_error_page(
                500,
                "内部服务器错误".to_string(),
                Some("模板渲染失败".to_string()),
            )
            .await
        }
        Err(e) => {
            error!("Template rendering task failed: {:?}", e);
            render_error_page(
                500,
                "内部服务器错误".to_string(),
                Some("任务执行失败".to_string()),
            )
            .await
        }
    }
}

/// Client detail page
pub async fn client_detail(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    info!("Client detail page accessed for client: {}", client_id);

    // Validate client_id format (should be UUID-like)
    if !is_valid_client_id(&client_id) {
        warn!("Invalid client_id format: {}", client_id);
        return render_error_page(400, "无效的客户端ID".to_string(), None).await;
    }

    if let Some(client_info) = state.client_manager.get_client(&client_id).await {
        let commands = state.client_manager.get_command_results(&client_id).await;

        // Convert ClientInfo to DisplayClientInfo
        let current_timestamp = chrono::Utc::now().timestamp();
        let is_online = (current_timestamp - client_info.last_seen.timestamp())
            < state.config.client_timeout as i64;
        let client = DisplayClientInfo {
            id: client_info.id,
            hostname: client_info.hostname,
            username: client_info.username,
            os: client_info.os,
            arch: client_info.arch,
            ip: client_info.ip,
            country_info: client_info.country_info,
            cpu_brand: client_info.cpu_brand,
            cpu_frequency: client_info.cpu_frequency,
            cpu_cores: client_info.cpu_cores,
            memory: client_info.memory,
            total_disk_space: client_info.total_disk_space,
            available_disk_space: client_info.available_disk_space,
            total_disk_space_gb: format!("{:.2}", client_info.total_disk_space as f64),
            available_disk_space_gb: format!("{:.2}", client_info.available_disk_space as f64),
            connected_at: client_info.connected_at,
            last_seen: client_info.last_seen,
            is_online,
        };

        let template = ClientTemplate {
            client,
            command_results: commands,
        };

        match tokio::task::spawn_blocking(move || template.render()).await {
            Ok(Ok(html)) => {
                info!(
                    "Client detail page rendered successfully for: {}",
                    client_id
                );
                Ok(Html(html))
            }
            Ok(Err(e)) => {
                error!(
                    "Template rendering failed for client {}: {:?}",
                    client_id, e
                );
                render_error_page(
                    500,
                    "内部服务器错误".to_string(),
                    Some("模板渲染失败".to_string()),
                )
                .await
            }
            Err(e) => {
                error!(
                    "Template rendering task failed for client {}: {:?}",
                    client_id, e
                );
                render_error_page(
                    500,
                    "内部服务器错误".to_string(),
                    Some("任务执行失败".to_string()),
                )
                .await
            }
        }
    } else {
        warn!("Client not found: {}", client_id);
        render_error_page(
            404,
            "客户端未找到".to_string(),
            Some(format!("客户端ID {client_id} 不存在")),
        )
        .await
    }
}

/// Validate client ID format
fn is_valid_client_id(client_id: &str) -> bool {
    // Check if it's a valid UUID format or at least reasonable length
    client_id.len() >= 8
        && client_id.len() <= 64
        && client_id.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// Render error page with proper logging
async fn render_error_page(
    status_code: u16,
    error_message: String,
    error_detail: Option<String>,
) -> Result<Html<String>, StatusCode> {
    let template = ErrorTemplate {
        error_code: status_code,
        error_message: error_message.clone(),
        error_detail: error_detail.clone(),
    };

    match tokio::task::spawn_blocking(move || template.render()).await {
        Ok(Ok(html)) => {
            error!("Error page rendered: {} - {}", status_code, error_message);
            if let Some(detail) = error_detail {
                error!("Error detail: {}", detail);
            }
            Ok(Html(html))
        }
        Ok(Err(e)) => {
            error!("Failed to render error page template: {:?}", e);
            // Fallback to basic HTML error page
            let fallback_html = format!(
                r#"<!DOCTYPE html>
                <html>
                <head><title>错误 {status_code}</title></head>
                <body>
                    <h1>错误 {status_code}</h1>
                    <p>{error_message}</p>
                </body>
                </html>"#
            );
            Ok(Html(fallback_html))
        }
        Err(e) => {
            error!("Error page rendering task failed: {:?}", e);
            // Fallback to basic HTML error page
            let fallback_html = format!(
                r#"<!DOCTYPE html>
                <html>
                <head><title>错误 {status_code}</title></head>
                <body>
                    <h1>错误 {status_code}</h1>
                    <p>{error_message}</p>
                </body>
                </html>"#
            );
            Ok(Html(fallback_html))
        }
    }
}
