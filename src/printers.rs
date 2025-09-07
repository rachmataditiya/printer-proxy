use crate::{
    config::{Backend, Config, Printer},
    errors::ProxyError,
    handlers::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::Path as FsPath,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{error, info, warn, instrument};

#[derive(Debug, Deserialize)]
pub struct PrinterCreateRequest {
    pub name: String,
    pub id: String,
    pub backend: Backend,
}

#[derive(Debug, Deserialize)]
pub struct PrinterUpdateRequest {
    pub name: Option<String>,
    pub backend: Option<Backend>,
}

#[derive(Debug, Serialize)]
pub struct PrinterResponse {
    pub name: String,
    pub id: String,
    pub backend: Backend,
}

#[derive(Debug, Serialize)]
pub struct PrintersListResponse {
    pub printers: Vec<PrinterResponse>,
    pub total: usize,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: String,
}

impl<T> ApiResponse<T> {
    fn success(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Validate admin token (reuse from admin module)
fn validate_admin_token(provided_token: Option<&str>) -> bool {
    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    
    if admin_token.is_empty() {
        warn!("⚠️ ADMIN_TOKEN not set - printer management disabled");
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

/// Get printers configuration file path
fn get_config_path() -> String {
    std::env::var("PRINTERS_CONFIG").unwrap_or_else(|_| "printers.yaml".to_string())
}

/// Load printers configuration from file
fn load_printers_config() -> Result<Config, ProxyError> {
    let config_path = get_config_path();
    
    if !FsPath::new(&config_path).exists() {
        return Err(ProxyError::BadPayload(format!("Configuration file not found: {}", config_path)));
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| ProxyError::Io(format!("Failed to read config file: {}", e)))?;
    
    serde_yaml::from_str(&content)
        .map_err(|e| ProxyError::BadPayload(format!("Invalid YAML configuration: {}", e)))
}

/// Save printers configuration to file atomically
fn save_printers_config(config: &Config) -> Result<(), ProxyError> {
    let config_path = get_config_path();
    let temp_path = format!("{}.tmp", config_path);
    
    // Serialize to YAML
    let yaml_content = serde_yaml::to_string(config)
        .map_err(|e| ProxyError::BadPayload(format!("Failed to serialize config: {}", e)))?;
    
    // Write to temporary file first
    fs::write(&temp_path, yaml_content)
        .map_err(|e| ProxyError::Io(format!("Failed to write temp config: {}", e)))?;
    
    // Atomic rename
    fs::rename(&temp_path, &config_path)
        .map_err(|e| {
            // Cleanup temp file on error
            let _ = fs::remove_file(&temp_path);
            ProxyError::Io(format!("Failed to save config: {}", e))
        })?;
    
    info!("✅ Configuration saved to {}", config_path);
    Ok(())
}

/// Reload printer configuration in memory
async fn reload_printer_config(state: &Arc<RwLock<AppState>>) -> Result<(), ProxyError> {
    let config = load_printers_config()?;
    let printers_map = config.printers.into_iter()
        .map(|p| (p.id.clone(), p))
        .collect::<HashMap<String, Printer>>();
    
    let mut app_state = state.write().await;
    app_state.printers = Arc::new(printers_map);
    
    info!("🔄 Printer configuration reloaded with {} printers", app_state.printers.len());
    Ok(())
}

/// List all printers
#[instrument(skip(state))]
pub async fn list_printers(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("📋 List printers request received");
    
    if !validate_admin_token(query.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for list printers");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<PrintersListResponse>::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    let app_state = state.read().await;
    let printers: Vec<PrinterResponse> = app_state.printers
        .values()
        .map(|p| PrinterResponse {
            name: p.name.clone(),
            id: p.id.clone(),
            backend: p.backend.clone(),
        })
        .collect();
    
    let response = PrintersListResponse {
        total: printers.len(),
        printers,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    Ok((
        StatusCode::OK,
        Json(ApiResponse::success("Printers retrieved successfully", response))
    ).into_response())
}

/// Get specific printer by ID
#[instrument(skip(state))]
pub async fn get_printer(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(printer_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🔍 Get printer request for ID: {}", printer_id);
    
    if !validate_admin_token(query.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for get printer");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<PrinterResponse>::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    let app_state = state.read().await;
    match app_state.printers.get(&printer_id) {
        Some(printer) => {
            let response = PrinterResponse {
                name: printer.name.clone(),
                id: printer.id.clone(),
                backend: printer.backend.clone(),
            };
            Ok((
                StatusCode::OK,
                Json(ApiResponse::success("Printer retrieved successfully", response))
            ).into_response())
        }
        None => {
            warn!("❌ Printer not found: {}", printer_id);
            Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<PrinterResponse>::error(format!("Printer '{}' not found", printer_id)))
            ).into_response())
        }
    }
}

/// Create new printer
#[instrument(skip(state))]
pub async fn create_printer(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(query): Query<HashMap<String, String>>,
    Json(request): Json<PrinterCreateRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("➕ Create printer request for ID: {}", request.id);
    
    if !validate_admin_token(query.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for create printer");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<PrinterResponse>::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    // Validate request
    if request.id.is_empty() || request.name.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<PrinterResponse>::error("ID and name are required"))
        ).into_response());
    }
    
    // Check if printer already exists
    {
        let app_state = state.read().await;
        if app_state.printers.contains_key(&request.id) {
            warn!("❌ Printer already exists: {}", request.id);
            return Ok((
                StatusCode::CONFLICT,
                Json(ApiResponse::<PrinterResponse>::error(format!("Printer '{}' already exists", request.id)))
            ).into_response());
        }
    }
    
    // Load current config
    let mut config = load_printers_config()
        .map_err(|e| {
            error!("❌ Failed to load config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Add new printer
    let new_printer = Printer {
        name: request.name.clone(),
        id: request.id.clone(),
        backend: request.backend.clone(),
    };
    
    config.printers.push(new_printer.clone());
    
    // Save config
    save_printers_config(&config)
        .map_err(|e| {
            error!("❌ Failed to save config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Reload in memory
    reload_printer_config(&state).await
        .map_err(|e| {
            error!("❌ Failed to reload config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let response = PrinterResponse {
        name: new_printer.name,
        id: new_printer.id,
        backend: new_printer.backend,
    };
    
    info!("✅ Printer created successfully: {}", request.id);
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success("Printer created successfully", response))
    ).into_response())
}

/// Update existing printer
#[instrument(skip(state))]
pub async fn update_printer(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(printer_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    Json(request): Json<PrinterUpdateRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("✏️ Update printer request for ID: {}", printer_id);
    
    if !validate_admin_token(query.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for update printer");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<PrinterResponse>::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    // Load current config
    let mut config = load_printers_config()
        .map_err(|e| {
            error!("❌ Failed to load config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Find and update printer
    let printer_index = config.printers.iter().position(|p| p.id == printer_id);
    match printer_index {
        Some(index) => {
            let printer = &mut config.printers[index];
            
            if let Some(name) = request.name {
                printer.name = name;
            }
            if let Some(backend) = request.backend {
                printer.backend = backend;
            }
            
            let updated_printer = printer.clone();
            
            // Save config
            save_printers_config(&config)
                .map_err(|e| {
                    error!("❌ Failed to save config: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            
            // Reload in memory
            reload_printer_config(&state).await
                .map_err(|e| {
                    error!("❌ Failed to reload config: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            
            let response = PrinterResponse {
                name: updated_printer.name,
                id: updated_printer.id,
                backend: updated_printer.backend,
            };
            
            info!("✅ Printer updated successfully: {}", printer_id);
            Ok((
                StatusCode::OK,
                Json(ApiResponse::success("Printer updated successfully", response))
            ).into_response())
        }
        None => {
            warn!("❌ Printer not found for update: {}", printer_id);
            Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<PrinterResponse>::error(format!("Printer '{}' not found", printer_id)))
            ).into_response())
        }
    }
}

/// Delete printer
#[instrument(skip(state))]
pub async fn delete_printer(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(printer_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🗑️ Delete printer request for ID: {}", printer_id);
    
    if !validate_admin_token(query.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for delete printer");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    // Load current config
    let mut config = load_printers_config()
        .map_err(|e| {
            error!("❌ Failed to load config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Find and remove printer
    let initial_count = config.printers.len();
    config.printers.retain(|p| p.id != printer_id);
    
    if config.printers.len() == initial_count {
        warn!("❌ Printer not found for deletion: {}", printer_id);
        return Ok((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error(format!("Printer '{}' not found", printer_id)))
        ).into_response());
    }
    
    // Save config
    save_printers_config(&config)
        .map_err(|e| {
            error!("❌ Failed to save config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Reload in memory
    reload_printer_config(&state).await
        .map_err(|e| {
            error!("❌ Failed to reload config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    info!("✅ Printer deleted successfully: {}", printer_id);
    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(format!("Printer '{}' deleted successfully", printer_id), ()))
    ).into_response())
}

/// Reload printer configuration from file
#[instrument(skip(state))]
pub async fn reload_printers(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("🔄 Reload printers configuration request");
    
    if !validate_admin_token(query.get("token").map(|s| s.as_str())) {
        warn!("❌ Invalid or missing admin token for reload printers");
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Invalid or missing admin token"))
        ).into_response());
    }
    
    // Reload configuration
    reload_printer_config(&state).await
        .map_err(|e| {
            error!("❌ Failed to reload config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let app_state = state.read().await;
    let printer_count = app_state.printers.len();
    
    info!("✅ Printers configuration reloaded with {} printers", printer_count);
    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(
            format!("Configuration reloaded successfully with {} printers", printer_count),
            ()
        ))
    ).into_response())
}
