// CheraghTunnel API - Tunnels Routes Submodule
use axum::{
    Json, Extension,
    response::{IntoResponse, Response},
    http::{StatusCode, header},
    extract::Multipart,
};
use std::sync::Arc;
use serde::Serialize;
use crate::api::AppState;
use crate::api::deploy::{run_ssh_command, generate_server_script, generate_client_script};
use crate::db::{self, Tunnel};

pub async fn telemetry_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match db::get_recent_telemetry_history(&state.db_path, id, 100) {
        Ok(logs) => (StatusCode::OK, Json(logs)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())).into_response(),
    }
}

pub async fn get_tunnels_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    match db::get_tunnels(&state.db_path) {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_tunnel_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<Tunnel>,
) -> impl IntoResponse {
    match db::get_tunnels(&state.db_path) {
        Ok(tunnels) => {
            for t in tunnels {
                if payload.iran_node_id.is_some() && t.iran_node_id == payload.iran_node_id {
                    if t.iran_port == payload.iran_port {
                        return (StatusCode::BAD_REQUEST, "Public port is already in use on this server").into_response();
                    }
                    if t.control_port == payload.control_port {
                        return (StatusCode::BAD_REQUEST, "Control port is already in use on this server").into_response();
                    }
                }
            }
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }

    match db::create_tunnel(&state.db_path, &payload) {
        Ok(id) => {
            let state_clone = state.clone();
            tokio::spawn(async move {
                if let Ok(Some(tunnel)) = db::get_tunnel_by_id(&state_clone.db_path, id) {
                    if let Some(i_id) = tunnel.iran_node_id {
                        if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                            let server_script = generate_server_script(&tunnel);
                            let cmd = "cat > /tmp/server.sh && bash /tmp/server.sh && rm -f /tmp/server.sh";
                            let _ = run_ssh_command(&n, cmd, Some(&server_script)).await;
                        }
                    }
                    if let Some(k_id) = tunnel.kharej_node_id {
                        if let Ok(Some(k_n)) = db::get_node_by_id(&state_clone.db_path, k_id) {
                            if let Some(i_id) = tunnel.iran_node_id {
                                if let Ok(Some(i_n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                                    let client_script = generate_client_script(&tunnel, &i_n.host);
                                    let cmd = "cat > /tmp/client.sh && bash /tmp/client.sh && rm -f /tmp/client.sh";
                                    let _ = run_ssh_command(&k_n, cmd, Some(&client_script)).await;
                                }
                            }
                        }
                    }
                }
            });
            (StatusCode::CREATED, Json(id)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_tunnel_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    if let Ok(Some(tunnel)) = db::get_tunnel_by_id(&state.db_path, id) {
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Some(i_id) = tunnel.iran_node_id {
                if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                    let cmd = format!(
                        "systemctl stop cheragh-server-{} || true && systemctl disable cheragh-server-{} || true && rm -f /etc/systemd/system/cheragh-server-{}.service && rm -f /usr/local/bin/cheraghtunnel-{} && systemctl daemon-reload",
                        id, id, id, id
                    );
                    let _ = run_ssh_command(&n, &cmd, None).await;
                }
            }
            if let Some(k_id) = tunnel.kharej_node_id {
                if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, k_id) {
                    let cmd = format!(
                        "systemctl stop cheragh-node-{} || true && systemctl disable cheragh-node-{} || true && rm -f /etc/systemd/system/cheragh-node-{}.service && rm -f /usr/local/bin/cheraghtunnel-{} && systemctl daemon-reload",
                        id, id, id, id
                    );
                    let _ = run_ssh_command(&n, &cmd, None).await;
                }
            }
        });
    }
    
    match db::delete_tunnel(&state.db_path, id) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_tunnel_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match db::get_tunnel_by_id(&state.db_path, id) {
        Ok(Some(t)) => (StatusCode::OK, Json(t)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_tunnel_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<Tunnel>,
) -> impl IntoResponse {
    let tunnel_opt = db::get_tunnel_by_id(&state.db_path, id).unwrap_or(None);
    let was_active = if let Some(t) = &tunnel_opt { t.status == "active" } else { false };
    
    let mut final_tunnel = payload;
    if let Some(ref existing) = tunnel_opt {
        final_tunnel.status = existing.status.clone();
        if final_tunnel.quota_used_bytes.unwrap_or(0) == 0 {
            final_tunnel.quota_used_bytes = existing.quota_used_bytes;
        }
        if final_tunnel.iran_node_id.is_none() {
            final_tunnel.iran_node_id = existing.iran_node_id;
        }
        if final_tunnel.kharej_node_id.is_none() {
            final_tunnel.kharej_node_id = existing.kharej_node_id;
        }

        // Only enforce port collision check if ports or Iran node actually changed
        let port_changed = final_tunnel.iran_port != existing.iran_port 
            || final_tunnel.control_port != existing.control_port 
            || final_tunnel.iran_node_id != existing.iran_node_id;

        if port_changed {
            if let Ok(tunnels) = db::get_tunnels(&state.db_path) {
                for t in tunnels {
                    if t.id != Some(id) && t.iran_node_id == final_tunnel.iran_node_id && final_tunnel.iran_node_id.is_some() {
                        if t.iran_port == final_tunnel.iran_port {
                            return (StatusCode::BAD_REQUEST, "Public port is already in use on this server").into_response();
                        }
                        if t.control_port == final_tunnel.control_port {
                            return (StatusCode::BAD_REQUEST, "Control port is already in use on this server").into_response();
                        }
                    }
                }
            }
        }
    }

    match db::update_tunnel(&state.db_path, id, &final_tunnel) {
        Ok(true) => {
            if was_active {
                let _ = db::update_tunnel_status(&state.db_path, id, "deploying");
                let state_clone = state.clone();
                tokio::spawn(async move {
                    if let Ok(Some(tunnel)) = db::get_tunnel_by_id(&state_clone.db_path, id) {
                        if let Some(i_id) = tunnel.iran_node_id {
                            if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                                let server_script = generate_server_script(&tunnel);
                                let cmd = "cat > /tmp/server.sh && bash /tmp/server.sh && rm -f /tmp/server.sh";
                                if let Err(e) = run_ssh_command(&n, cmd, Some(&server_script)).await {
                                    eprintln!("[DEPLOY] Iran Node SSH failed during update: {}", e);
                                    let _ = db::update_tunnel_status(&state_clone.db_path, id, "error");
                                    return;
                                }
                            }
                        }
                        if let Some(k_id) = tunnel.kharej_node_id {
                            if let Ok(Some(k_n)) = db::get_node_by_id(&state_clone.db_path, k_id) {
                                if let Some(i_id) = tunnel.iran_node_id {
                                    if let Ok(Some(i_n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                                        let client_script = generate_client_script(&tunnel, &i_n.host);
                                        let cmd = "cat > /tmp/client.sh && bash /tmp/client.sh && rm -f /tmp/client.sh";
                                        if let Err(e) = run_ssh_command(&k_n, cmd, Some(&client_script)).await {
                                            eprintln!("[DEPLOY] Kharej Node SSH failed during update: {}", e);
                                            let _ = db::update_tunnel_status(&state_clone.db_path, id, "error");
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                        let _ = db::update_tunnel_status(&state_clone.db_path, id, "active");
                    }
                });
            }
            StatusCode::OK.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "Tunnel ID not found in database").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn toggle_tunnel_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let tunnel_opt = db::get_tunnel_by_id(&state.db_path, id).unwrap_or(None);
    let tunnel = match tunnel_opt {
        Some(t) => t,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if tunnel.status == "active" {
        let _ = db::update_tunnel_status(&state.db_path, id, "inactive");
        
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Some(i_id) = tunnel.iran_node_id {
                if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, i_id) {
                    let _ = run_ssh_command(&n, &format!("systemctl disable cheragh-server-{} && systemctl stop cheragh-server-{}", tunnel.id.unwrap(), tunnel.id.unwrap()), None).await;
                }
            }
            if let Some(k_id) = tunnel.kharej_node_id {
                if let Ok(Some(n)) = db::get_node_by_id(&state_clone.db_path, k_id) {
                    let _ = run_ssh_command(&n, &format!("systemctl disable cheragh-node-{} && systemctl stop cheragh-node-{}", tunnel.id.unwrap(), tunnel.id.unwrap()), None).await;
                }
            }
        });
        StatusCode::OK.into_response()
    } else {
        // Find saved Iran and Kharej nodes
        let iran_node_id = match tunnel.iran_node_id {
            Some(id) => id,
            None => return (StatusCode::BAD_REQUEST, "Iran Node is not selected for this tunnel").into_response(),
        };
        let kharej_node_id = match tunnel.kharej_node_id {
            Some(id) => id,
            None => return (StatusCode::BAD_REQUEST, "Kharej Node is not selected for this tunnel").into_response(),
        };

        let iran_node = match db::get_node_by_id(&state.db_path, iran_node_id).unwrap_or(None) {
            Some(n) => n,
            None => return (StatusCode::BAD_REQUEST, "Selected Iran Node not found").into_response(),
        };
        let kharej_node = match db::get_node_by_id(&state.db_path, kharej_node_id).unwrap_or(None) {
            Some(n) => n,
            None => return (StatusCode::BAD_REQUEST, "Selected Kharej Node not found").into_response(),
        };

        let _ = db::update_tunnel_status(&state.db_path, id, "deploying");
        let db_path_spawn = state.db_path.clone();

        tokio::spawn(async move {
            // Deploy Iran Server
            let server_script = generate_server_script(&tunnel);
            let cmd = "cat > /tmp/server.sh && bash /tmp/server.sh && rm -f /tmp/server.sh";
            if let Err(e) = run_ssh_command(&iran_node, cmd, Some(&server_script)).await {
                eprintln!("[DEPLOY] Iran Node SSH failed: {}", e);
                let _ = db::update_tunnel_status(&db_path_spawn, id, "error");
                return;
            }

            // Deploy Kharej Client
            let client_script = generate_client_script(&tunnel, &iran_node.host);
            let cmd = "cat > /tmp/client.sh && bash /tmp/client.sh && rm -f /tmp/client.sh";
            if let Err(e) = run_ssh_command(&kharej_node, cmd, Some(&client_script)).await {
                eprintln!("[DEPLOY] Kharej Node SSH failed: {}", e);
                let _ = db::update_tunnel_status(&db_path_spawn, id, "error");
                return;
            }

            let _ = db::update_tunnel_status(&db_path_spawn, id, "active");
        });

        StatusCode::OK.into_response()
    }
}

#[derive(Serialize)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub mem_usage: f32,
    pub active_tunnels: usize,
    pub total_tunnels: usize,
}

pub async fn stats_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let sys = state.system_monitor.lock().await;

    let cpu_usage = sys.global_cpu_info().cpu_usage();
    let total_mem = sys.total_memory();
    let mem_usage = if total_mem > 0 {
        (sys.used_memory() as f32 / total_mem as f32) * 100.0
    } else {
        0.0
    };
    drop(sys);
    
    let tunnels = db::get_tunnels(&state.db_path).unwrap_or_default();
    let total_tunnels = tunnels.len();
    let active_tunnels = tunnels.iter().filter(|t| t.status == "active").count();

    Json(SystemStats {
        cpu_usage,
        mem_usage,
        active_tunnels,
        total_tunnels,
    })
}

pub async fn backup_handler(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    match tokio::fs::read(&state.db_path).await {
        Ok(data) => {
            let len = data.len();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"cheragh_backup.sqlite\"")
                .header(header::CONTENT_LENGTH, len)
                .body(axum::body::Body::from(data))
                .unwrap()
        }
        Err(e) => {
            eprintln!("[API] Failed to read database for backup: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from("Failed to generate backup"))
                .unwrap()
        }
    }
}

pub async fn restore_handler(
    Extension(state): Extension<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => return (StatusCode::BAD_REQUEST, format!("Failed to read upload: {}", e)).into_response(),
        };

        let tmp_path = std::env::temp_dir().join("cheragh_restore_tmp.sqlite");
        if let Err(e) = tokio::fs::write(&tmp_path, &data).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save temp file: {}", e)).into_response();
        }

        if let Err(e) = rusqlite::Connection::open(&tmp_path) {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return (StatusCode::BAD_REQUEST, format!("Invalid SQLite file: {}", e)).into_response();
        }

        if let Err(e) = tokio::fs::copy(&tmp_path, &state.db_path).await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to restore database: {}", e)).into_response();
        }
        let _ = tokio::fs::remove_file(&tmp_path).await;

        return StatusCode::OK.into_response();
    }
    
    (StatusCode::BAD_REQUEST, "No file uploaded".to_string()).into_response()
}

pub async fn ws_telemetry_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws_telemetry(socket, state))
}

async fn handle_ws_telemetry(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        let tunnels = db::get_tunnels(&state.db_path).unwrap_or_default();
        let nodes = db::get_nodes(&state.db_path).unwrap_or_default();

        let sys = state.system_monitor.lock().await;
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let total_mem = sys.total_memory();
        let mem_usage = if total_mem > 0 {
            (sys.used_memory() as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        drop(sys);
        
        let payload = serde_json::json!({
            "type": "telemetry_update",
            "cpu_usage": cpu_usage,
            "mem_usage": mem_usage,
            "tunnels": tunnels,
            "nodes": nodes,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

        if socket.send(axum::extract::ws::Message::Text(payload.to_string())).await.is_err() {
            break;
        }
    }
}
