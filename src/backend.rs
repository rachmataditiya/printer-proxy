use crate::{config::Printer, errors::ProxyError, pool::CONNECTION_MANAGER};
use tracing::{instrument};

/// Send payload to printer backend using connection pool
#[instrument(skip(payload), fields(payload_size = payload.len()))]
pub async fn send_to_backend(printer: &Printer, payload: &[u8]) -> Result<(), ProxyError> {
    CONNECTION_MANAGER.send_to_printer(printer, payload).await
}
