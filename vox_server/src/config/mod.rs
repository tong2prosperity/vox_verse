
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use config::{Config, ConfigError, File};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub signaling_server: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RtcConfig {
    
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub log: LogConfig,
    pub rtc: RtcConfig,
}

// 全局配置实例
pub static CONFIG: Lazy<Arc<RwLock<AppConfig>>> = Lazy::new(|| {
    Arc::new(RwLock::new(AppConfig::default()))
});

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
              signaling_server: "wss://your-signaling-server.com/room".to_string(),
            },
            log: LogConfig {
                level: "info".to_string(), 
                path: "./logs".to_string(),
            },
            rtc: RtcConfig {
            },
        }
    }
}



impl AppConfig {
    pub async fn load(config_path: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name(config_path))
            .build()?;
            
        config.try_deserialize()
    }

    pub async fn reload(config_path: &str) -> Result<(), ConfigError> {
        let new_config = Self::load(config_path).await?;
        let mut config = CONFIG.write().await;
        *config = new_config;
        Ok(())
    }

    pub fn watch_config(config_path: String) -> notify::Result<()> {
        let path = config_path.clone();
        let mut watcher = RecommendedWatcher::new(move|res| {
            let path = path.clone();
            match res {
                Ok(_) => {
                    tokio::spawn(async move {
                        if let Err(e) = AppConfig::reload(&path).await {
                            eprintln!("Failed to reload config: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Watch error: {}", e),
            }
        }, notify::Config::default())?;

        watcher.watch(Path::new(&config_path), RecursiveMode::NonRecursive)?;
        
        // 保持 watcher 存活
        std::mem::forget(watcher);
        
        Ok(())
    }
}