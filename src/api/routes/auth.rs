// CheraghTunnel API - Auth Routes Submodule
use axum::{
    Json, Extension,
    response::IntoResponse,
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
) -> impl IntoResponse {
    // Rate limit check: block after 5 failed attempts within 60 seconds
    if !state.login_limiter.check_and_record() {
        return Json(LoginResponse {
            success: false,
            token: None,
            message: "Too many login attempts. Please wait 60 seconds.".to_string(),
        });
    }

    let db_username = db::get_setting(&state.db_path, "admin_username")
        .unwrap_or(Some("admin".to_string()))
        .unwrap_or("admin".to_string());
    let db_password = db::get_setting(&state.db_path, "admin_password")
        .unwrap_or(None)
        .unwrap_or_default();

    if constant_time_eq(payload.username.as_bytes(), db_username.as_bytes()) && db::verify_password(&payload.password, &db_password) {
        // Reset rate limiter on successful login
        state.login_limiter.reset();

        // Generate a cryptographically random session token
        let token = format!("{:016x}{:016x}", rand::random::<u64>(), rand::random::<u64>());
        
        // Store it in shared state
        let mut session = state.session_token.lock().await;
        *session = Some(token.clone());
        
        Json(LoginResponse {
            success: true,
            token: Some(token),
            message: "Login successful".to_string(),
        })
    } else {
        Json(LoginResponse {
            success: false,
            token: None,
            message: "Invalid credentials".to_string(),
        })
    }
}
