// CheraghTunnel API - Nodes Routes Submodule
use axum::{
    Json, Extension,
    response::IntoResponse,
    http::{StatusCode, HeaderMap, header},
};
use std::sync::Arc;
use crate::api::AppState;
use crate::api::deploy::generate_client_script;
use crate::db;

pub async fn get_nodes_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    match db::get_nodes(&state.db_path) {
        Ok(nodes) => (StatusCode::OK, Json(nodes)).into_response(),
        Err(e) => {
            eprintln!("[API] Error fetching nodes: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_node_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<db::Node>,
) -> impl IntoResponse {
    match db::create_node(&state.db_path, &payload) {
        Ok(id) => (StatusCode::OK, Json(id)).into_response(),
        Err(e) => {
            eprintln!("[API] Error creating node: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_node_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match db::get_node_by_id(&state.db_path, id) {
        Ok(Some(node)) => (StatusCode::OK, Json(node)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_node_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<db::Node>,
) -> impl IntoResponse {
    match db::update_node(&state.db_path, id, &payload) {
        Ok(true) => StatusCode::OK.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_node_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match db::delete_node(&state.db_path, id) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            eprintln!("[API] Error deleting node: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn node_script_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let tunnel_opt = db::get_tunnel_by_id(&state.db_path, id).unwrap_or(None);
    let tunnel = match tunnel_opt {
        Some(t) => t,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    // Get host from request headers to auto-fill Iran server IP
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1")
        .split(':')
        .next()
        .unwrap_or("127.0.0.1");

    let script = generate_client_script(&tunnel, host);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/x-sh"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"node.sh\""),
        ],
        script,
    ).into_response()
}
