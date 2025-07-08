use crate::state::AppState;
use askama::Template;
use axum::{
    extract::{Form, Request, State},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use rand::Rng;
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

/// 生成32位随机字母数字字符串
pub fn generate_session_token() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();

    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn login_get() -> Html<String> {
    let template = LoginTemplate {};
    Html(template.render().unwrap())
}

pub async fn login_post(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(credentials): Form<Credentials>,
) -> impl IntoResponse {
    const USERNAME: &str = "Rust-Admin";
    const PASSWORD: &str = "Passwd@RustC2";

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
