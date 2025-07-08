use axum::{
    extract::{Path, State},
    response::Html,
    http::StatusCode,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use common::*;
use crate::state::ServerState;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub clients: Vec<DisplayClientInfo>,
    pub online_clients_count: usize,
    pub os_types_count: usize,
}

#[derive(Template)]
#[template(path = "client.html")]
pub struct ClientTemplate {
    pub client: ClientInfo,
    pub commands: Vec<CommandResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayClientInfo {
    #[serde(flatten)]
    pub client_info: ClientInfo,
    pub is_online: bool,
}

/// 主页面
pub async fn index(State(state): State<ServerState>) -> Result<Html<String>, StatusCode> {
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
pub async fn client_detail(
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
