use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error")]
    ConfigError,

    #[error("Serde YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Parse command error")]
    ParseCommandError,

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[error("Unrecoverable error on {0}")]
    Fatal(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
