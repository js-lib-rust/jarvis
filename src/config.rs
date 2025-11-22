use std::{fs, sync::OnceLock};

use log::trace;
use serde::Deserialize;

use crate::error::{AppError, Result};

pub static CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn init_config(path: &str) -> Result<()> {
    trace!("config::init_config(path: &str) -> Result<()>");
    let config = AppConfig::load(path);
    CONFIG.set(config).map_err(|_| AppError::ConfigError)
}

pub fn get_config() -> &'static AppConfig {
    CONFIG.get().expect("Config not initialized")
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub slm_url: Option<String>,
    pub rag_system: Option<String>,
    pub prompt_system: Option<String>,
    pub user_profile: Option<String>,
    pub system_settings: Option<String>,
}

impl AppConfig {
    pub fn load(path: &str) -> Self {
        if let Ok(yaml) = fs::read_to_string(&path) {
            if let Ok(config) = serde_yaml::from_str(&yaml) {
                return config;
            }
        }
        AppConfig::default()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            slm_url: None,
            rag_system: None,
            prompt_system: None,
            user_profile: None,
            system_settings: None,
        }
    }
}
