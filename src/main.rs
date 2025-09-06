mod backend;
mod config;
mod errors;
mod escpos;
mod handlers;
mod health;
mod pool;

use axum::{
    routing::{any, get},
    Router, serve,
};
use config::{load_config, build_printers_map};
use handlers::{AppState, handle_print, health_check, printers_health_check, printer_health_check};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::{non_blocking, rolling};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup file logging dengan rotasi harian
    let file_appender = rolling::daily("logs", "printer-proxy.log");
    let (non_blocking_file, _guard) = non_blocking(file_appender);
    
    // Setup console logging
    let (non_blocking_stdout, _stdout_guard) = non_blocking(std::io::stdout());
    
    // Environment filter untuk level logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("printer_proxy=info,axum=info,tower_http=info"));
    
    // Kombinasi file dan console logging
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::Layer::new()
                .with_writer(non_blocking_stdout)
                .with_ansi(true)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(
            fmt::Layer::new()
                .with_writer(non_blocking_file)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .init();
    
    info!("🚀 Starting Printer Proxy (ESC/POS) v{}", env!("CARGO_PKG_VERSION"));
    info!("📝 Logs akan disimpan di folder: ./logs/");
    
    // Pastikan _guard tidak di-drop (untuk file logging)
    std::mem::forget(_guard);
    std::mem::forget(_stdout_guard);

    let config_path = std::env::var("PRINTERS_CONFIG").unwrap_or_else(|_| "printers.yaml".to_string());
    info!("📄 Loading config dari: {}", config_path);
    
    let config = load_config(&config_path)?;
    let printers_map = build_printers_map(config);

    if printers_map.is_empty() {
        error!("❌ Config tidak berisi printer apa pun");
        anyhow::bail!("Config tidak berisi printer apa pun");
    }
    
    info!("✅ Loaded {} printer(s) dari {}", printers_map.len(), config_path);
    for (id, printer) in &printers_map {
        info!("🖨️  Printer '{}' -> {:?}", id, printer.backend);
    }

    let state = AppState {
        printers: Arc::new(printers_map),
    };

    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/health/printers", get(printers_health_check))
        .route("/health/printer/:printer_id", get(printer_health_check))
        // Endpoint kompatibel ePOS: /:printer_id/cgi-bin/epos/service.cgi
        .route("/:printer_id/cgi-bin/epos/service.cgi", any(handle_print))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .expect("LISTEN_ADDR invalid");
    
    info!("🌐 Server akan listen di: http://{}", addr);
    info!("🔗 Health check: http://{}/healthz", addr);
    info!("🏥 Printers health: http://{}/health/printers", addr);
    info!("🏥 Individual health: http://{}/health/printer/{{printer_id}}", addr);
    info!("🖨️  Print endpoint: http://{}/{{printer_id}}/cgi-bin/epos/service.cgi", addr);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| {
            error!("❌ Failed to bind to address {}: {}", addr, e);
            e
        })
        .expect("Failed to bind to address");
    
    info!("✅ Server siap menerima koneksi di {}", addr);
    
    // Start background cleanup task
    tokio::spawn(async {
        pool::start_cleanup_task().await;
    });
    info!("🧹 Background cleanup task started");
    
    if let Err(e) = serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await {
        error!("❌ Server error: {}", e);
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install signal handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("🔄 Ctrl+C signal diterima, shutting down gracefully...");
        },
        _ = terminate => {
            warn!("🔄 SIGTERM signal diterima, shutting down gracefully...");
        },
    }
    info!("👋 Server stopped");
}