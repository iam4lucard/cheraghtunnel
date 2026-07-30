// CheraghTunnel API Module v1.32.0
pub mod deploy;
pub mod routes;

use axum::{
    routing::{get, post},
    Router, Extension,
    response::{IntoResponse, Response},
    http::{StatusCode, header},
    middleware::{self, Next},
    extract::Request,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use tokio::sync::Mutex;
use rust_embed::RustEmbed;
use sysinfo::System;

use crate::db;
use routes::auth::login_handler;
use routes::nodes::{get_nodes_handler, get_node_handler, create_node_handler, update_node_handler, delete_node_handler, node_script_handler};
use routes::tunnels::{
    get_tunnels_handler, create_tunnel_handler, get_tunnel_handler, update_tunnel_handler,
    delete_tunnel_handler, toggle_tunnel_handler, telemetry_handler, stats_handler,
    backup_handler, restore_handler, ws_telemetry_handler,
};

/// Constant-time byte comparison to prevent timing side-channel attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

/// Simple in-memory login rate limiter.
pub struct LoginRateLimiter {
    attempt_count: AtomicU32,
    window_start: AtomicU64,
}

impl LoginRateLimiter {
    const MAX_ATTEMPTS: u32 = 5;
    const WINDOW_SECS: u64 = 60;

    fn new() -> Self {
        Self {
            attempt_count: AtomicU32::new(0),
            window_start: AtomicU64::new(0),
        }
    }

    pub fn check_and_record(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window = self.window_start.load(Ordering::SeqCst);

        if now.saturating_sub(window) > Self::WINDOW_SECS {
            self.window_start.store(now, Ordering::SeqCst);
            self.attempt_count.store(1, Ordering::SeqCst);
            return true;
        }

        let count = self.attempt_count.fetch_add(1, Ordering::SeqCst) + 1;
        count <= Self::MAX_ATTEMPTS
    }

    pub fn reset(&self) {
        self.attempt_count.store(0, Ordering::SeqCst);
    }
}

pub struct AppState {
    pub db_path: PathBuf,
    pub session_token: Mutex<Option<String>>,
    pub system_monitor: Mutex<System>,
    pub login_limiter: LoginRateLimiter,
}

pub async fn run_panel(
    port: u16,
    db_path: PathBuf,
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sys = System::new_all();
    sys.refresh_cpu();
    sys.refresh_memory();

    let state = Arc::new(AppState {
        db_path: db_path.clone(),
        session_token: Mutex::new(None),
        system_monitor: Mutex::new(sys),
        login_limiter: LoginRateLimiter::new(),
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let mut sys = state_clone.system_monitor.lock().await;
            sys.refresh_cpu();
            sys.refresh_memory();
        }
    });
    
    // Spawn background telemetry fetcher & quota/expiry monitor
    let db_path_clone = db_path.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            if let Ok(tunnels) = db::get_tunnels(&db_path_clone) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                for t in tunnels {
                    if t.status == "active" || t.status == "unreachable" {
                        if let Some(exp) = t.expires_at {
                            if exp > 0 && now >= exp {
                                println!("[PANEL] Tunnel '{}' (ID {:?}) expired. Shutting down...", t.name, t.id);
                                let _ = db::update_tunnel_status(&db_path_clone, t.id.unwrap(), "expired");
                                continue;
                            }
                        }
                        let quota_limit = t.quota_limit_bytes.unwrap_or(0);
                        if t.status == "deploying" {
                            continue;
                        }

                        let quota_used = t.quota_used_bytes.unwrap_or(0);
                        if quota_limit > 0 && quota_used >= quota_limit {
                            println!("[PANEL] Tunnel '{}' (ID {:?}) quota limit reached. Shutting down...", t.name, t.id);
                            let _ = db::update_tunnel_status(&db_path_clone, t.id.unwrap(), "quota_exceeded");
                            continue;
                        }

                        let api_port = 18000 + t.id.unwrap_or(0) as u16;
                        let iran_host = if let Some(i_id) = t.iran_node_id {
                            if let Ok(Some(i_node)) = db::get_node_by_id(&db_path_clone, i_id) {
                                i_node.host
                            } else {
                                "127.0.0.1".to_string()
                            }
                        } else {
                            "127.0.0.1".to_string()
                        };
                        let url = format!("http://{}:{}/api/stats", iran_host, api_port);
                        
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(2))
                            .build()
                            .unwrap_or_default();

                        let mut cur_rx = 0u64;
                        let mut cur_tx = 0u64;
                        let mut measured_rtt: Option<f64> = None;
                        let mut api_responded = false;

                        if let Ok(resp) = client.get(&url).send().await {
                            if let Ok(json) = resp.json::<serde_json::Value>().await {
                                api_responded = true;
                                let rx_delta = json["rx_delta"].as_u64().unwrap_or(0);
                                let tx_delta = json["tx_delta"].as_u64().unwrap_or(0);
                                cur_rx = json["speed_rx"].as_u64().unwrap_or(0);
                                cur_tx = json["speed_tx"].as_u64().unwrap_or(0);
                                
                                let rtt_val = json["rtt_ms"].as_f64().unwrap_or(0.0);
                                if rtt_val > 0.0 && rtt_val < 999.0 {
                                    measured_rtt = Some(rtt_val);
                                }
                                
                                let _ = db::update_tunnel_speeds(&db_path_clone, t.id.unwrap(), rx_delta, tx_delta, cur_rx, cur_tx);
                            }
                        }

                        if measured_rtt.is_none() {
                            if let Some(k_id) = t.kharej_node_id {
                                if let Ok(Some(k_node)) = db::get_node_by_id(&db_path_clone, k_id) {
                                    measured_rtt = measure_tcp_ping(&k_node.host, t.control_port).await;
                                    if measured_rtt.is_none() {
                                        measured_rtt = measure_tcp_ping(&k_node.host, t.kharej_port).await;
                                    }
                                }
                            }
                        }

                        if let Some(rtt) = measured_rtt {
                            let _ = db::log_telemetry(&db_path_clone, t.id.unwrap(), rtt, 0.0, cur_rx, cur_tx);
                            let _ = db::update_tunnel_probe(&db_path_clone, t.id.unwrap(), "active", rtt);
                        } else if api_responded || cur_rx > 0 || cur_tx > 0 || t.stats_speed_rx > 0 || t.stats_speed_tx > 0 {
                            let _ = db::log_telemetry(&db_path_clone, t.id.unwrap(), 0.0, 0.0, cur_rx, cur_tx);
                            let _ = db::update_tunnel_probe(&db_path_clone, t.id.unwrap(), "active", 0.0);
                        } else {
                            let _ = db::log_telemetry(&db_path_clone, t.id.unwrap(), 999.0, 100.0, 0, 0);
                            let _ = db::update_tunnel_probe(&db_path_clone, t.id.unwrap(), "unreachable", 999.0);
                        }
                    }
                }
            }
        }
    });

    // Spawn background Node Health Checker & Automatic Failover Worker
    let db_path_node_check = db_path.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            if let Ok(nodes) = db::get_nodes(&db_path_node_check) {
                for node in nodes {
                    if let Some(node_id) = node.id {
                        let addr = format!("{}:{}", node.host, node.port);
                        let start = std::time::Instant::now();
                        let (status, latency) = match tokio::time::timeout(
                            tokio::time::Duration::from_secs(3),
                            tokio::net::TcpStream::connect(&addr),
                        ).await {
                            Ok(Ok(_)) => ("active", start.elapsed().as_secs_f64() * 1000.0),
                            _ => ("unreachable", 999.0),
                        };
                        let _ = db::update_node_health(&db_path_node_check, node_id, status, latency);

                        if status == "unreachable" && (node.role == "kharej" || node.role == "both") {
                            if let Ok(tunnels) = db::get_tunnels(&db_path_node_check) {
                                for mut t in tunnels {
                                    if t.kharej_node_id == Some(node_id) && t.status == "active" {
                                        let backup_nodes: Vec<_> = db::get_nodes(&db_path_node_check)
                                            .unwrap_or_default()
                                            .into_iter()
                                            .filter(|n| n.id != Some(node_id) && n.status.as_deref() == Some("active") && (n.role == "kharej" || n.role == "both"))
                                            .collect();
                                        
                                        if let Some(backup) = backup_nodes.first() {
                                            println!("[FAILOVER] Kharej Node '{}' is unreachable. Failing over Tunnel '{}' to backup Node '{}'", node.name, t.name, backup.name);
                                            t.kharej_node_id = backup.id;
                                            let _ = db::update_tunnel(&db_path_node_check, t.id.unwrap(), &t);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let public_routes = Router::new()
        .route("/", get(static_handler))
        .route("/index.html", get(static_handler))
        .route("/style.css", get(static_handler))
        .route("/app.js", get(static_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/ws/telemetry", get(ws_telemetry_handler))
        .route("/api/tunnels/:id/node-script", get(node_script_handler));

    let protected_routes = Router::new()
        .route("/api/tunnels", get(get_tunnels_handler).post(create_tunnel_handler))
        .route("/api/tunnels/:id", get(get_tunnel_handler).put(update_tunnel_handler).delete(delete_tunnel_handler))
        .route("/api/tunnels/:id/toggle", post(toggle_tunnel_handler))
        .route("/api/tunnels/:id/telemetry", get(telemetry_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/status", get(stats_handler))
        .route("/api/nodes", get(get_nodes_handler).post(create_node_handler))
        .route("/api/nodes/:id", get(get_node_handler).put(update_node_handler).delete(delete_node_handler))
        .route("/api/backup", get(backup_handler))
        .route("/api/restore", post(restore_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = public_routes
        .merge(protected_routes)
        .layer(Extension(state));

    if let (Some(cert), Some(key)) = (cert_path, key_path) {
        let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
        println!("Web Panel UI (HTTPS) available at: https://0.0.0.0:{}", port);
        axum_server::bind_rustls(format!("0.0.0.0:{}", port).parse()?, config)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        println!("Web Panel UI (HTTP) available at: http://127.0.0.1:{}", port);
        axum::serve(listener, app).await?;
    }
    Ok(())
}

async fn measure_tcp_ping(host: &str, port: u16) -> Option<f64> {
    let addr = format!("{}:{}", host, port);
    let start = std::time::Instant::now();
    match tokio::time::timeout(
        std::time::Duration::from_millis(2000),
        tokio::net::TcpStream::connect(&addr)
    ).await {
        Ok(Ok(_stream)) => {
            let rtt = start.elapsed().as_secs_f64() * 1000.0;
            Some(rtt)
        }
        _ => None,
    }
}

async fn auth_middleware(
    Extension(state): Extension<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token_lock = state.session_token.lock().await;

    let req_token = if let Some(auth) = req.headers().get("authorization").and_then(|v| v.to_str().ok()) {
        auth.trim_start_matches("Bearer ").trim().to_string()
    } else if let Some(q) = req.uri().query() {
        q.split('&')
            .find(|p| p.starts_with("token="))
            .map(|p| p.trim_start_matches("token=").to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    if !req_token.is_empty() {
        let memory_match = if let Some(ref valid_token) = *token_lock {
            constant_time_eq(req_token.as_bytes(), valid_token.as_bytes())
        } else {
            false
        };

        if memory_match || db::is_session_valid(&state.db_path, &req_token) {
            drop(token_lock);
            return Ok(next.run(req).await);
        }
    }

    drop(token_lock);
    Err(StatusCode::UNAUTHORIZED)
}

async fn static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() || path == "index.html" {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, max-age=0")
                .body(axum::body::Body::from(content.data))
                .unwrap()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
