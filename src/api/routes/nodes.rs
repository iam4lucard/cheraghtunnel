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
        Ok(true) => {
            // Automatically redeploy any active tunnels using this updated node
            let state_clone = state.clone();
            tokio::spawn(async move {
                if let Ok(tunnels) = db::get_tunnels_by_node_id(&state_clone.db_path, id) {
                    for tunnel in tunnels {
                        if tunnel.status == "active" {
                            let _ = db::update_tunnel_status(&state_clone.db_path, tunnel.id.unwrap(), "deploying");
                            if let Some(i_id) = tunnel.iran_node_id {
                                if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                                    let server_script = crate::api::deploy::generate_server_script(&tunnel);
                                    let cmd = "cat > /tmp/server.sh && bash /tmp/server.sh && rm -f /tmp/server.sh";
                                    if let Err(e) = crate::api::deploy::run_ssh_command(&n, cmd, Some(&server_script)).await {
                                        eprintln!("[DEPLOY] Iran Node SSH failed on node update: {}", e);
                                        let _ = db::update_tunnel_status(&state_clone.db_path, tunnel.id.unwrap(), "error");
                                        continue;
                                    }
                                }
                            }
                            if let Some(k_id) = tunnel.kharej_node_id {
                                if let Ok(Some(k_n)) = db::get_node_by_id(&state_clone.db_path, k_id) {
                                    if let Some(i_id) = tunnel.iran_node_id {
                                        if let Ok(Some(i_n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                                            let client_script = crate::api::deploy::generate_client_script(&tunnel, &i_n.host);
                                            let cmd = "cat > /tmp/client.sh && bash /tmp/client.sh && rm -f /tmp/client.sh";
                                            if let Err(e) = crate::api::deploy::run_ssh_command(&k_n, cmd, Some(&client_script)).await {
                                                eprintln!("[DEPLOY] Kharej Node SSH failed on node update: {}", e);
                                                let _ = db::update_tunnel_status(&state_clone.db_path, tunnel.id.unwrap(), "error");
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            let _ = db::update_tunnel_status(&state_clone.db_path, tunnel.id.unwrap(), "active");
                        }
                    }
                }
            });
            StatusCode::OK.into_response()
        }
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

    // Check saved Iran node IP first to prevent proxy/localhost issues
    let mut host_ip = String::new();
    if let Some(i_id) = tunnel.iran_node_id {
        if let Ok(Some(n)) = db::get_node_by_id(&state.db_path, i_id) {
            if !n.host.is_empty() && n.host != "127.0.0.1" && n.host != "localhost" {
                host_ip = n.host;
            }
        }
    }

    if host_ip.is_empty() {
        let header_host = headers
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("127.0.0.1")
            .split(':')
            .next()
            .unwrap_or("127.0.0.1");
        host_ip = header_host.to_string();
    }

    let script = generate_client_script(&tunnel, &host_ip);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/x-sh"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"node.sh\""),
        ],
        script,
    ).into_response()
}
