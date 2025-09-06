use crate::{
    config::{Backend, Printer},
    errors::ProxyError,
    health::PrinterStatus,
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::{Mutex, Semaphore},
    time::timeout,
};
use tracing::{debug, error, info, instrument};

/// Connection pool entry
#[derive(Debug)]
struct PooledConnection {
    stream: TcpStream,
    created_at: Instant,
    last_used: Instant,
}

impl PooledConnection {
    fn new(stream: TcpStream) -> Self {
        let now = Instant::now();
        Self {
            stream,
            created_at: now,
            last_used: now,
        }
    }

    fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    fn is_idle_too_long(&self, max_idle: Duration) -> bool {
        self.last_used.elapsed() > max_idle
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }
}

/// Connection pool for a specific printer
#[derive(Debug)]
struct PrinterPool {
    connections: Mutex<Vec<PooledConnection>>,
    #[allow(dead_code)]
    semaphore: Arc<Semaphore>,
    max_connections: usize,
    max_age: Duration,
    max_idle: Duration,
}

impl PrinterPool {
    fn new(max_connections: usize) -> Self {
        Self {
            connections: Mutex::new(Vec::with_capacity(max_connections)),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            max_connections,
            max_age: Duration::from_secs(300), // 5 minutes
            max_idle: Duration::from_secs(60), // 1 minute
        }
    }

    async fn get_connection(&self, addr: &str) -> Result<TcpStream, ProxyError> {
        // Try to get an existing connection first
        {
            let mut connections = self.connections.lock().await;
            while let Some(mut conn) = connections.pop() {
                if !conn.is_expired(self.max_age) && !conn.is_idle_too_long(self.max_idle) {
                    conn.mark_used();
                    debug!("🔄 Reusing pooled connection to {}", addr);
                    return Ok(conn.stream);
                }
                debug!("🗑️ Discarding expired/idle connection to {}", addr);
            }
        }

        // No valid connection available, create new one
        debug!("🔌 Creating new connection to {}", addr);
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| {
                error!("❌ TCP connect to {} failed: {}", addr, e);
                ProxyError::Io(format!("TCP connect {} gagal: {}", addr, e))
            })?;

        Ok(stream)
    }

    async fn return_connection(&self, stream: TcpStream) {
        let mut connections = self.connections.lock().await;
        if connections.len() < self.max_connections {
            connections.push(PooledConnection::new(stream));
            debug!("📥 Returned connection to pool (total: {})", connections.len());
        } else {
            debug!("🗑️ Pool full, dropping connection");
        }
    }

    async fn cleanup_expired(&self) {
        let mut connections = self.connections.lock().await;
        let initial_count = connections.len();
        connections.retain(|conn| {
            !conn.is_expired(self.max_age) && !conn.is_idle_too_long(self.max_idle)
        });
        let removed = initial_count - connections.len();
        if removed > 0 {
            debug!("🧹 Cleaned up {} expired connections", removed);
        }
    }
}

/// Global connection pool manager
#[derive(Debug)]
pub struct ConnectionManager {
    pools: DashMap<String, Arc<PrinterPool>>,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            pools: DashMap::new(),
        }
    }

    fn get_pool(&self, addr: &str) -> Arc<PrinterPool> {
        self.pools
            .entry(addr.to_string())
            .or_insert_with(|| Arc::new(PrinterPool::new(5))) // Max 5 connections per printer
            .clone()
    }

    pub async fn send_to_printer(&self, printer: &Printer, payload: &[u8]) -> Result<(), ProxyError> {
        let Backend::Tcp9100 { host, port } = &printer.backend;
        let addr = format!("{}:{}", host, port);
        
        let pool = self.get_pool(&addr);
        let mut stream = pool.get_connection(&addr).await?;

        info!("📦 Sending {} bytes to {}", payload.len(), addr);
        debug!("📦 Payload preview: {:02X?}", &payload[..payload.len().min(32)]);

        let result = async {
            stream.write_all(payload).await?;
            stream.flush().await?;
            Ok::<(), std::io::Error>(())
        }.await;

        match result {
            Ok(()) => {
                info!("✅ Successfully sent {} bytes to {}", payload.len(), addr);
                // Return connection to pool for reuse
                pool.return_connection(stream).await;
                Ok(())
            }
            Err(e) => {
                error!("❌ TCP write/flush to {} failed: {}", addr, e);
                // Don't return failed connection to pool
                Err(ProxyError::Io(format!("TCP write {} gagal: {}", addr, e)))
            }
        }
    }

    pub async fn cleanup_all_pools(&self) {
        for entry in self.pools.iter() {
            entry.value().cleanup_expired().await;
        }
    }
}

/// Global connection manager instance
pub static CONNECTION_MANAGER: Lazy<ConnectionManager> = Lazy::new(ConnectionManager::new);

/// Health check cache entry
#[derive(Debug, Clone)]
struct HealthCacheEntry {
    status: PrinterStatus,
    timestamp: Instant,
}

impl HealthCacheEntry {
    fn new(status: PrinterStatus) -> Self {
        Self {
            status,
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }
}

/// Health check cache manager
#[derive(Debug)]
pub struct HealthCache {
    cache: DashMap<String, HealthCacheEntry>,
    ttl: Duration,
}

impl HealthCache {
    fn new(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }

    pub async fn get_or_check(&self, printer: &Printer) -> PrinterStatus {
        let cache_key = format!("{}:{}", printer.id, match &printer.backend {
            Backend::Tcp9100 { host, port } => format!("{}:{}", host, port),
        });

        // Try cache first
        if let Some(entry) = self.cache.get(&cache_key) {
            if !entry.is_expired(self.ttl) {
                debug!("💾 Health cache hit for {}", cache_key);
                return entry.status.clone();
            }
            debug!("⏰ Health cache expired for {}", cache_key);
        }

        // Cache miss or expired, perform actual health check
        debug!("🔍 Performing health check for {}", cache_key);
        let status = self.check_printer_health_direct(printer).await;
        
        // Update cache
        self.cache.insert(cache_key, HealthCacheEntry::new(status.clone()));
        
        status
    }

    #[instrument(skip(self, printer), fields(printer_id = %printer.id))]
    async fn check_printer_health_direct(&self, printer: &Printer) -> PrinterStatus {
        let Backend::Tcp9100 { host, port } = &printer.backend;
        let addr = format!("{}:{}", host, port);
        
        debug!("🔍 Direct TCP health check for {}", addr);
        
        // Quick connection test with short timeout
        let check_result = timeout(
            Duration::from_millis(1500), // Reduced from 2 seconds
            TcpStream::connect(&addr)
        ).await;
        
        match check_result {
            Ok(Ok(_stream)) => {
                debug!("✅ TCP health check passed for {}", addr);
                PrinterStatus::Online
            }
            Ok(Err(e)) => {
                debug!("❌ TCP health check failed for {}: {}", addr, e);
                PrinterStatus::Offline
            }
            Err(_timeout) => {
                debug!("⏰ TCP health check timeout for {}", addr);
                PrinterStatus::Offline
            }
        }
    }

    #[allow(dead_code)]
    pub fn invalidate(&self, printer: &Printer) {
        let cache_key = format!("{}:{}", printer.id, match &printer.backend {
            Backend::Tcp9100 { host, port } => format!("{}:{}", host, port),
        });
        self.cache.remove(&cache_key);
        debug!("🗑️ Invalidated health cache for {}", cache_key);
    }

    pub async fn cleanup_expired(&self) {
        let initial_count = self.cache.len();
        self.cache.retain(|_, entry| !entry.is_expired(self.ttl));
        let removed = initial_count - self.cache.len();
        if removed > 0 {
            debug!("🧹 Cleaned up {} expired health cache entries", removed);
        }
    }
}

/// Global health cache instance with 30 second TTL
pub static HEALTH_CACHE: Lazy<HealthCache> = Lazy::new(|| HealthCache::new(Duration::from_secs(30)));

/// Background task to cleanup expired connections and cache entries
pub async fn start_cleanup_task() {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Cleanup every minute
    
    loop {
        interval.tick().await;
        debug!("🧹 Running background cleanup task");
        
        // Cleanup connection pools
        CONNECTION_MANAGER.cleanup_all_pools().await;
        
        // Cleanup health cache
        HEALTH_CACHE.cleanup_expired().await;
    }
}
