use crate::{error::AppError, types::Result};
use serde::Deserialize;
use std::{fs, sync::OnceLock};

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) username: String,
}

impl Config {
    pub(crate) fn load(path: &str) -> Result<()> {
        let content = fs::read_to_string(path).map_err(|error| {
            AppError::Fatal(format!("Can't read config file {}. Error: {}", path, error))
        })?;
        CONFIG.set(serde_yaml::from_str::<Config>(&content)?).ok();
        Ok(())
    }

    pub(crate) fn get() -> &'static Config {
        CONFIG.get().expect("Config must be loaded before use!")
    }
}
