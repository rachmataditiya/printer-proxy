use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use thiserror::Error;
use tracing::{error, debug};

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Printer '{0}' tidak ditemukan")]
    NotFound(String),
    #[error("Printer '{0}' sedang offline dan tidak dapat menerima request")]
    PrinterOffline(String),
    #[error("Backend tidak didukung untuk printer '{0}'")]
    #[allow(dead_code)]
    Unsupported(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Payload tidak valid: {0}")]
    BadPayload(String),
    #[error("Kesalahan internal")]
    #[allow(dead_code)]
    Internal,
}

/* === Uniform XML responses (persis seperti Python) === */

fn cors_headers_xml() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("text/xml"));
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("POST, OPTIONS"),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("Content-Type"),
    );
    headers
}

pub fn xml_success() -> impl IntoResponse {
    debug!("✅ Returning XML success response");
    let headers = cors_headers_xml();
    (
        StatusCode::OK,
        headers,
        "<?xml version=\"1.0\"?><response success=\"true\" code=\"0\"/>",
    )
}

pub fn xml_error() -> impl IntoResponse {
    debug!("❌ Returning XML error response");
    let headers = cors_headers_xml();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        "<?xml version=\"1.0\"?><response success=\"false\" code=\"1\"/>",
    )
}

pub fn xml_options_no_content() -> impl IntoResponse {
    debug!("🔄 Returning OPTIONS no-content response");
    let headers = cors_headers_xml();
    (StatusCode::NO_CONTENT, headers, "")
}

/* Return error ke client SELALU dengan XML error seperti Python */
impl IntoResponse for ProxyError {
    fn into_response(self) -> axum::response::Response {
        error!("Request error: {self}");
        xml_error().into_response()
    }
}
