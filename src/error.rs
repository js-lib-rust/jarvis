use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON Serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Timeout: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("Chrono: {0}")]
    Chrono(#[from] chrono::ParseError),

    #[error("Template: {0}")]
    Template(#[from] askama::Error),

    #[error("Parse Integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("MongoDB: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[error("MongoDB: {0}")]
    MongoSer(#[from] mongodb::bson::ser::Error),

    #[error("Fatal error: {0}")]
    Fatal(String),

    #[allow(dead_code)]
    #[error("Shutdown")]
    Shutdown,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, error_message).into_response()
    }
}
