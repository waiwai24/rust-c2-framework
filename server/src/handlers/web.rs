use axum::{
    extract::{Path, State},
    response::Html,
    http::StatusCode,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use common::message::{ClientInfo, CommandResponse};

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

/// Main dashboard page
pub async fn index(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let clients = state.client_manager.get_clients().await;
    let current_timestamp = chrono::Utc::now().timestamp();

    let display_clients: Vec<DisplayClientInfo> = clients
        .into_iter()
        .map(|c| {
            let is_online = (current_timestamp - c.last_seen.timestamp()) < state.config.client_timeout as i64;
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

/// Client detail page
pub async fn client_detail(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    if let Some(client) = state.client_manager.get_client(&client_id).await {
        let commands = state.client_manager.get_command_results(&client_id).await;

        let template = ClientTemplate {
            client,
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
