use crate::{
    backend::send_to_backend,
    config::Printer,
    errors::{ProxyError, xml_success, xml_options_no_content},
    escpos::{
        JsonJob, parse_epos_soap, build_escpos_from_epos_doc, build_escpos_from_ops,
        parse_bool_public, parse_bit_order_public,
    },
    health::{ensure_printer_online, check_printer_health, PrinterStatus},
};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, Method},
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use http::header::CONTENT_TYPE;
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn, error, debug, instrument};
use serde_json::json;

#[derive(Clone)]
pub struct AppState {
    pub printers: Arc<HashMap<String, Printer>>,
}

#[instrument(skip(state, body), fields(printer_id = %printer_id, method = %method, content_length = body.len()))]
pub async fn handle_print(
    State(state): State<AppState>,
    Path(printer_id): Path<String>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<impl IntoResponse, ProxyError> {
    info!("📥 Incoming request: {} {}", method, printer_id);
    // Preflight
    if method == Method::OPTIONS {
        debug!("🔄 Handling OPTIONS preflight request");
        return Ok(xml_options_no_content().into_response());
    }

    if method != Method::POST && method != Method::PUT {
        warn!("❌ Invalid method: {} (only POST/PUT allowed)", method);
        return Err(ProxyError::BadPayload("Gunakan POST/PUT untuk kirim data cetak".into()));
    }

    let printer = state
        .printers
        .get(&printer_id)
        .ok_or_else(|| {
            error!("❌ Printer '{}' tidak ditemukan", printer_id);
            ProxyError::NotFound(printer_id.clone())
        })?;
    
    info!("✅ Printer '{}' ditemukan: {:?}", printer_id, printer.backend);
    
    // Health check sebelum processing request
    info!("🔍 Checking printer '{}' health status...", printer_id);
    ensure_printer_online(printer).await?;
    info!("✅ Printer '{}' is online and ready", printer_id);

    // Override opsional (query/header)
    let invert_override = query.get("invert")
        .and_then(|v| Some(parse_bool_public(v)))
        .or_else(|| headers.get("x-escpos-invert").and_then(|h| h.to_str().ok()).map(parse_bool_public));

    let bit_override = query.get("bit")
        .map(|v| parse_bit_order_public(v))
        .or_else(|| headers.get("x-escpos-bit-order").and_then(|h| h.to_str().ok()).map(parse_bit_order_public));

    // Content-Type
    let ct = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    
    debug!("📄 Content-Type: {}", ct);
    debug!("📊 Body size: {} bytes", body.len());

    // Mode A: ePOS SOAP
    if ct.starts_with("text/plain")
        || ct.starts_with("text/xml")
        || ct.starts_with("application/xml")
    {
        info!("🔄 Processing ePOS-Print SOAP mode");
        let doc = parse_epos_soap(&body, invert_override, bit_override)?;
        info!("✅ Parsed {} image(s), cut: {:?}", doc.images.len(), doc.cut);
        
        let bytes = build_escpos_from_epos_doc(&doc)?;
        info!("📦 Generated {} ESC/POS bytes", bytes.len());
        
        send_to_backend(printer, &bytes).await?;
        info!("✅ Successfully sent to printer '{}'", printer_id);
        return Ok(xml_success().into_response());
    }

    // Mode B: RAW ESC/POS
    if ct.starts_with("application/octet-stream")
        || headers
            .get("x-esc-pos-mode")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("raw"))
    {
        info!("🔄 Processing RAW ESC/POS mode");
        if body.is_empty() {
            warn!("❌ Empty body for raw mode");
            return Err(ProxyError::BadPayload("Body kosong untuk mode raw".into()));
        }
        
        info!("📦 Sending {} raw bytes to printer", body.len());
        send_to_backend(printer, &body).await?;
        info!("✅ Successfully sent raw data to printer '{}'", printer_id);
        return Ok(xml_success().into_response());
    }

    // Mode C: JSON job
    if ct.starts_with("application/json") {
        info!("🔄 Processing JSON job mode");
        let job: JsonJob =
            serde_json::from_slice(&body).map_err(|e| {
                error!("❌ JSON parsing error: {}", e);
                ProxyError::BadPayload(format!("JSON invalid: {e}"))
            })?;
            
        let bytes = match job {
            JsonJob::RawBase64 { ref base64 } => {
                info!("📦 Processing base64 data ({} chars)", base64.len());
                BASE64_STANDARD.decode(base64).map_err(|e| {
                    error!("❌ Base64 decode error: {}", e);
                    ProxyError::BadPayload(format!("Base64 invalid: {e}"))
                })?
            }
            JsonJob::Ops { ref ops } => {
                info!("🔄 Processing {} operations", ops.len());
                for (i, op) in ops.iter().enumerate() {
                    debug!("  Op {}: {:?}", i, op);
                }
                build_escpos_from_ops(&ops)?
            }
        };
        
        if bytes.is_empty() {
            warn!("❌ Generated empty ESC/POS data");
            return Err(ProxyError::BadPayload("Tidak ada data ESC/POS yang akan dikirim".into()));
        }
        
        info!("📦 Generated {} ESC/POS bytes from JSON", bytes.len());
        send_to_backend(printer, &bytes).await?;
        info!("✅ Successfully sent JSON job to printer '{}'", printer_id);
        return Ok(xml_success().into_response());
    }

    warn!("❌ Unsupported content type: {}", ct);
    Err(ProxyError::BadPayload(
        "Unsupported payload. Gunakan text/plain|text/xml|application/xml (ePOS), application/octet-stream (raw), atau application/json (job).".into(),
    ))
}

#[instrument]
pub async fn health_check() -> &'static str {
    debug!("❤️ Health check requested");
    "ok"
}

/// Check health status of all printers
#[instrument(skip(state))]
pub async fn printers_health_check(State(state): State<AppState>) -> impl IntoResponse {
    info!("🏥 Checking health status of all printers");
    
    let mut results = HashMap::new();
    let mut futures = Vec::new();
    
    // Create futures for all printer health checks
    for (id, printer) in state.printers.iter() {
        let printer_clone = printer.clone();
        let id_clone = id.clone();
        
        futures.push(async move {
            let status = check_printer_health(&printer_clone).await;
            (id_clone, status)
        });
    }
    
    // Execute all health checks concurrently
    let health_results = futures::future::join_all(futures).await;
    
    let mut online_count = 0;
    let mut offline_count = 0;
    
    for (id, status) in health_results {
        let status_str = match status {
            PrinterStatus::Online => {
                online_count += 1;
                "online"
            }
            PrinterStatus::Offline => {
                offline_count += 1;
                "offline"
            }
            PrinterStatus::Unknown => "unknown"
        };
        
        results.insert(id, json!({
            "status": status_str,
            "message": status.to_string()
        }));
    }
    
    let overall_status = if offline_count == 0 { "healthy" } else { "degraded" };
    
    info!("🏥 Health check completed: {} online, {} offline", online_count, offline_count);
    
    let response = json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "total": state.printers.len(),
            "online": online_count,
            "offline": offline_count
        },
        "printers": results
    });
    
    axum::Json(response)
}

/// Check health status of a specific printer
#[instrument(skip(state))]
pub async fn printer_health_check(
    State(state): State<AppState>,
    Path(printer_id): Path<String>,
) -> Result<impl IntoResponse, ProxyError> {
    info!("🏥 Checking health status of printer '{}'", printer_id);
    
    let printer = state
        .printers
        .get(&printer_id)
        .ok_or_else(|| ProxyError::NotFound(printer_id.clone()))?;
    
    let status = check_printer_health(printer).await;
    
    let status_str = match status {
        PrinterStatus::Online => "online",
        PrinterStatus::Offline => "offline",
        PrinterStatus::Unknown => "unknown",
    };
    
    info!("🏥 Printer '{}' status: {}", printer_id, status);
    
    let response = json!({
        "printer_id": printer_id,
        "status": status_str,
        "message": status.to_string(),
        "backend": printer.backend,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(axum::Json(response))
}
