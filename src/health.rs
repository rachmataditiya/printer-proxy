use crate::{config::{Printer, Backend}, errors::ProxyError};
use std::time::Duration;
use tokio::{net::TcpStream, time::timeout};
use tracing::{info, warn, debug, instrument};

#[derive(Debug, Clone, PartialEq)]
pub enum PrinterStatus {
    Online,
    Offline,
    #[allow(dead_code)]
    Unknown,
}

impl std::fmt::Display for PrinterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrinterStatus::Online => write!(f, "🟢 Online"),
            PrinterStatus::Offline => write!(f, "🔴 Offline"),
            PrinterStatus::Unknown => write!(f, "🟡 Unknown"),
        }
    }
}

/// Check if a printer is reachable
#[instrument(skip(printer), fields(printer_id = %printer.id))]
pub async fn check_printer_health(printer: &Printer) -> PrinterStatus {
    match &printer.backend {
        Backend::Tcp9100 { host, port } => {
            check_tcp_health(host, *port).await
        }
    }
}

/// Check TCP connectivity to printer
#[instrument]
async fn check_tcp_health(host: &str, port: u16) -> PrinterStatus {
    let addr = format!("{}:{}", host, port);
    debug!("🔍 Checking TCP health for {}", addr);
    
    // Set a reasonable timeout for health check (2 seconds)
    let check_result = timeout(
        Duration::from_secs(2),
        TcpStream::connect(&addr)
    ).await;
    
    match check_result {
        Ok(Ok(_stream)) => {
            info!("✅ TCP health check passed for {}", addr);
            PrinterStatus::Online
        }
        Ok(Err(e)) => {
            warn!("❌ TCP health check failed for {}: {}", addr, e);
            PrinterStatus::Offline
        }
        Err(_timeout) => {
            warn!("⏰ TCP health check timeout for {}", addr);
            PrinterStatus::Offline
        }
    }
}

/// Validate printer is online before processing request
#[instrument(skip(printer), fields(printer_id = %printer.id))]
pub async fn ensure_printer_online(printer: &Printer) -> Result<(), ProxyError> {
    let status = check_printer_health(printer).await;
    
    match status {
        PrinterStatus::Online => {
            debug!("✅ Printer '{}' is online, proceeding with request", printer.id);
            Ok(())
        }
        PrinterStatus::Offline => {
            warn!("❌ Printer '{}' is offline, rejecting request", printer.id);
            Err(ProxyError::PrinterOffline(printer.id.clone()))
        }
        PrinterStatus::Unknown => {
            warn!("⚠️ Printer '{}' status unknown, proceeding with caution", printer.id);
            Ok(()) // Allow unknown status to pass through
        }
    }
}

/// Quick health check without detailed logging (for bulk checks)
#[allow(dead_code)]
pub async fn quick_health_check(printer: &Printer) -> PrinterStatus {
    match &printer.backend {
        Backend::Tcp9100 { host, port } => {
            let addr = format!("{}:{}", host, port);
            
            match timeout(Duration::from_millis(500), TcpStream::connect(&addr)).await {
                Ok(Ok(_)) => PrinterStatus::Online,
                _ => PrinterStatus::Offline,
            }
        }
    }
}
