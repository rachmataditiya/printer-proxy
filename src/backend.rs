use crate::{config::{Printer, Backend}, errors::ProxyError};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tracing::{info, error, debug, instrument};

/// Send payload to printer backend
#[instrument(skip(payload), fields(payload_size = payload.len()))]
pub async fn send_to_backend(printer: &Printer, payload: &[u8]) -> Result<(), ProxyError> {
    match &printer.backend {
        Backend::Tcp9100 { host, port } => {
            let addr = format!("{}:{}", host, port);
            info!("🔌 Connecting to printer at {}", addr);
            
            let mut stream = TcpStream::connect(&addr)
                .await
                .map_err(|e| {
                    error!("❌ TCP connect to {} failed: {}", addr, e);
                    ProxyError::Io(format!("TCP connect {} gagal: {}", addr, e))
                })?;
            
            info!("✅ Connected to {}, sending {} bytes", addr, payload.len());
            debug!("📦 Payload preview: {:02X?}", &payload[..payload.len().min(32)]);
            
            stream
                .write_all(payload)
                .await
                .map_err(|e| {
                    error!("❌ TCP write to {} failed: {}", addr, e);
                    ProxyError::Io(format!("TCP write {} gagal: {}", addr, e))
                })?;
            
            stream
                .flush()
                .await
                .map_err(|e| {
                    error!("❌ TCP flush to {} failed: {}", addr, e);
                    ProxyError::Io(format!("TCP flush {} gagal: {}", addr, e))
                })?;
            
            info!("🎯 Successfully sent {} bytes to {}", payload.len(), addr);
            Ok(())
        }
    }
}
