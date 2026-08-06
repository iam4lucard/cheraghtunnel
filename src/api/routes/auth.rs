// CheraghTunnel API - Auth Routes Submodule
use axum::{
    Json, Extension,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::api::{AppState, constant_time_eq};
use crate::db;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub message: String,
}

pub async fn login_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Response {
    // Rate limit check: block after 5 failed attempts within 60 seconds
    if !state.login_limiter.check_and_record() {
        return Json(LoginResponse {
            success: false,
            token: None,
            message: "Too many login attempts. Please wait 60 seconds.".to_string(),
        }).into_response();
    }

    let db_path = state.db_path.clone();
    let db_username = tokio::task::spawn_blocking({
        let path = db_path.clone();
        move || {
            db::get_setting(&path, "admin_username")
                .unwrap_or(Some("admin".to_string()))
                .unwrap_or("admin".to_string())
        }
    }).await.unwrap_or_else(|_| "admin".to_string());

    let db_password = tokio::task::spawn_blocking({
        let path = db_path.clone();
        move || db::get_setting(&path, "admin_password").unwrap_or(None).unwrap_or_default()
    }).await.unwrap_or_default();

    if constant_time_eq(payload.username.as_bytes(), db_username.as_bytes()) && db::verify_password(&payload.password, &db_password) {
        // Transparent password upgrade to Argon2id if legacy SHA-256 hash stored
        if !db_password.starts_with("$argon2id$") {
            let new_hash = db::hash_password(&payload.password);
            let path = db_path.clone();
            let _ = tokio::task::spawn_blocking(move || db::set_setting(&path, "admin_password", &new_hash)).await;
        }

        // Reset rate limiter on successful login
        state.login_limiter.reset();

        // Generate a cryptographically random session token (256-bit CSPRNG)
        let mut token_bytes = [0u8; 32];
        {
            use rand::RngCore;
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut token_bytes);
        }
        let token = token_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        // Store session token in SQLite DB
        let path = db_path.clone();
        let tok_clone = token.clone();
        let _ = tokio::task::spawn_blocking(move || db::create_session(&path, &tok_clone)).await;

        Json(LoginResponse {
            success: true,
            token: Some(token),
            message: "Login successful".to_string(),
        }).into_response()
    } else {
        Json(LoginResponse {
            success: false,
            token: None,
            message: "Invalid credentials".to_string(),
        }).into_response()
    }
}
