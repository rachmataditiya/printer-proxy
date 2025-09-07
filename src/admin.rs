use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, process::Command, time::Duration};
use tokio::time::sleep;
use tracing::{error, info, warn, instrument};

use crate::handlers::AppState;

#[derive(Debug, Deserialize)]
pub struct AdminQuery {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminResponse {
    success: bool,
    message: String,
    timestamp: String,
}

impl AdminResponse {
    fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Validate admin token from environment variable
fn validate_admin_token(provided_token: Option<&str>) -> bool {
    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    
    if admin_token.is_empty() {
        warn!("⚠️ ADMIN_TOKEN not set - admin endpoints disabled");
        return false;
    }
    
    if admin_token.len() < 16 {
        warn!("⚠️ ADMIN_TOKEN too short (minimum 16 characters)");
        return false;
    }
    
    match provided_token {
        Some(token) => token == admin_token,
        None => false,
    }
}

/// Admin shutdown endpoint
#[instrument(skip(_state))]
pub async fn admin_shutdown(
    State(_state): State<AppState>,
    Query(query): Query<AdminQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🔒 Admin shutdown request received");
    
    if !validate_admin_token(query.token.as_deref()) {
        warn!("❌ Invalid or missing admin token for shutdown");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(AdminResponse::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    info!("🛑 Initiating graceful shutdown...");
    
    // Schedule shutdown after responding to client
    tokio::spawn(async {
        sleep(Duration::from_secs(2)).await;
        info!("👋 Shutting down printer proxy...");
        std::process::exit(0);
    });
    
    Ok((
        StatusCode::OK,
        Json(AdminResponse::success("Graceful shutdown initiated - server will stop in 2 seconds"))
    ).into_response())
}

/// Admin restart endpoint  
#[instrument(skip(_state))]
pub async fn admin_restart(
    State(_state): State<AppState>,
    Query(query): Query<AdminQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🔒 Admin restart request received");
    
    if !validate_admin_token(query.token.as_deref()) {
        warn!("❌ Invalid or missing admin token for restart");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(AdminResponse::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    info!("🔄 Initiating service restart...");
    
    // Try to restart via systemctl if running as service
    tokio::spawn(async {
        sleep(Duration::from_secs(1)).await;
        
        // First try systemctl restart
        let restart_result = Command::new("systemctl")
            .args(["restart", "printer-proxy"])
            .output();
            
        match restart_result {
            Ok(output) => {
                if output.status.success() {
                    info!("✅ Service restart initiated via systemctl");
                } else {
                    warn!("⚠️ systemctl restart failed, attempting graceful shutdown");
                    // Fallback to shutdown (systemd will restart if configured)
                    std::process::exit(1);
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to execute systemctl: {}, falling back to exit", e);
                std::process::exit(1);
            }
        }
    });
    
    Ok((
        StatusCode::OK,
        Json(AdminResponse::success("Service restart initiated - attempting systemctl restart"))
    ).into_response())
}

/// Admin SSL renewal endpoint
#[instrument(skip(_state))]
pub async fn admin_renew_ssl(
    State(_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🔒 Admin SSL renewal request received");
    
    if !validate_admin_token(params.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for SSL renewal");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(AdminResponse::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    let domain = params.get("domain").cloned().unwrap_or_else(|| "localhost".to_string());
    let port = params.get("port").cloned().unwrap_or_else(|| "8080".to_string());
    
    info!("🔐 Starting SSL renewal for domain: {}", domain);
    
    // Execute SSL renewal script
    let ssl_script_path = "./ssl.sh";
    
    // Check if ssl.sh exists
    if !std::path::Path::new(ssl_script_path).exists() {
        error!("❌ SSL script not found at: {}", ssl_script_path);
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AdminResponse::error("SSL renewal script not found"))
        ).into_response());
    }
    
    // Clone for use in response message
    let domain_clone = domain.clone();
    let port_clone = port.clone();
    
    // Execute SSL renewal in background
    tokio::spawn(async move {
        info!("🔄 Executing SSL renewal script...");
        
        let output = Command::new("sudo")
            .args([ssl_script_path, &domain, &port])
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    info!("✅ SSL renewal completed successfully");
                    info!("📄 SSL script output: {}", stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    error!("❌ SSL renewal failed with exit code: {}", result.status.code().unwrap_or(-1));
                    error!("📄 SSL script error: {}", stderr);
                }
            }
            Err(e) => {
                error!("❌ Failed to execute SSL renewal script: {}", e);
            }
        }
    });
    
    Ok((
        StatusCode::OK,
        Json(AdminResponse::success(format!(
            "SSL renewal initiated for domain '{}' on port '{}' - check logs for progress",
            domain_clone, port_clone
        )))
    ).into_response())
}

/// Admin status endpoint
#[instrument(skip(state))]
pub async fn admin_status(
    State(state): State<AppState>,
    Query(query): Query<AdminQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🔒 Admin status request received");
    
    if !validate_admin_token(query.token.as_deref()) {
        warn!("❌ Invalid or missing admin token for status");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(AdminResponse::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let status = json!({
        "success": true,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": {
            "name": "printer-proxy",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": uptime,
            "printers_configured": state.printers.len()
        },
        "system": {
            "pid": std::process::id(),
            "memory_usage": get_memory_usage(),
        },
        "endpoints": {
            "health": "/healthz",
            "printers_health": "/health/printers", 
            "admin_shutdown": "/admin/shutdown?token=TOKEN",
            "admin_restart": "/admin/restart?token=TOKEN",
            "admin_ssl_renew": "/admin/ssl/renew?token=TOKEN&domain=DOMAIN&port=PORT",
            "admin_status": "/admin/status?token=TOKEN"
        }
    });
    
    Ok((StatusCode::OK, Json(status)).into_response())
}

/// Get approximate memory usage (Linux only)
fn get_memory_usage() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    return line.split_whitespace().nth(1)
                        .map(|s| format!("{} kB", s))
                        .unwrap_or_else(|| "unknown".to_string());
                }
            }
        }
    }
    "unknown".to_string()
}
