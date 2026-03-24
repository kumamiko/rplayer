use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Music folder to scan
    pub music_folder: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_folder: String::new(),
        }
    }
}

impl Config {
    /// Get config file path (same directory as executable)
    fn config_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("config.toml")
    }
    
    /// Load config from file, create default if not exists
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
