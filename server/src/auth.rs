use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Form, Request, State},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use rand::{rngs::OsRng, Rng};
use serde::Deserialize;
use tower_cookies::{Cookie, Cookies};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Generate a 32-character alphanumeric token for session management.
pub fn generate_session_token() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    // Discard thread_rng and use OsRng for better randomness in production.
    let mut rng = OsRng;

    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Render the login page.
pub async fn login_get() -> Html<String> {
    let template = LoginTemplate {};
    Html(template.render().unwrap())
}

/// Handle login POST request, validate credentials, and set session cookie.
pub async fn login_post(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(credentials): Form<Credentials>,
) -> impl IntoResponse {
    let username = state.config.auth.username.clone();
    let password = state.config.auth.password.clone();

    if credentials.username == username && credentials.password == password {
        // Generate session token
        let session_token = generate_session_token();
        // Set expiration time for the session token (e.g., 24 hours)
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        // Store session token
        {
            let mut tokens = state.session_tokens.write().await;
            tokens.insert(session_token.clone(), expires_at);
        }

        // Set cookie with session token
        let mut cookie = Cookie::new("session_token", session_token);
        cookie.set_max_age(Some(tower_cookies::cookie::time::Duration::hours(24)));
        cookie.set_http_only(true);
        // Set secure to false for development; should be true(https) in production
        cookie.set_secure(false);
        cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);

        cookies.add(cookie);
        axum::response::Redirect::to("/").into_response()
    } else {
        axum::response::Redirect::to("/login").into_response()
    }
}

/// Middleware to check if the user is authenticated by verifying the session token.
pub async fn auth_middleware(
    State(state): State<AppState>,
    cookies: Cookies,
    request: Request,
    next: Next,
) -> Response {
    if let Some(cookie) = cookies.get("session_token") {
        let token = cookie.value();
        let mut tokens = state.session_tokens.write().await;

        if let Some(expires_at) = tokens.get(token) {
            if chrono::Utc::now() < *expires_at {
                // Token is valid, proceed with the request
                return next.run(request).await;
            } else {
                // Token has expired, remove it
                tokens.remove(token);
            }
        }
    }
    axum::response::Redirect::to("/login").into_response()
}
