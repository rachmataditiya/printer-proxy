use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tracing::{info, debug, instrument};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub printers: Vec<Printer>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Printer {
    #[allow(dead_code)]
    pub name: String,
    pub id: String,
    pub backend: Backend,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum Backend {
    #[serde(rename = "tcp9100")]
    Tcp9100 { host: String, port: u16 },
}

#[instrument]
pub fn load_config(path: &str) -> anyhow::Result<Config> {
    debug!("📂 Reading config file: {}", path);
    let p = PathBuf::from(path);
    let bytes = std::fs::read(&p)?;
    debug!("📊 Config file size: {} bytes", bytes.len());
    
    let cfg: Config = serde_yaml::from_slice(&bytes)?;
    info!("✅ Successfully parsed config with {} printer(s)", cfg.printers.len());
    
    Ok(cfg)
}

#[instrument(skip(config))]
pub fn build_printers_map(config: Config) -> HashMap<String, Printer> {
    let printer_count = config.printers.len();
    let map = config.printers.into_iter().map(|p| {
        debug!("📋 Mapping printer: {} -> {}", p.id, p.name);
        (p.id.clone(), p)
    }).collect();
    
    info!("🗺️  Built printer map with {} entries", printer_count);
    map
}
