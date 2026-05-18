use crate::error::AppError;
use crate::llm_router::LlmRouterClient;
use axum::body::Bytes;
use axum::response::sse::Event;
use futures::StreamExt;
use futures::{TryStreamExt, stream::Stream};
use reqwest::Response;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct AppContext {
    pub(crate) router_client: LlmRouterClient,
    pub(crate) http_client: reqwest::Client,
    pub(crate) model_url: String,
}

impl AppContext {
    pub(crate) async fn create(router_addr: &str, model_url: &str) -> Result<Arc<AppContext>> {
        let router_client = LlmRouterClient::connect(router_addr).await?;
        let http_client = reqwest::Client::new();
        let model_url = model_url.to_string();
        Ok(Arc::new(AppContext { router_client, http_client, model_url }))
    }
}

pub(crate) type Result<T> = std::result::Result<T, AppError>;
pub(crate) type StringStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;
pub(crate) type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;
pub(crate) type EventStream = Pin<Box<dyn Stream<Item = Result<Event>> + Send>>;

pub(crate) trait ResponseExt {
    fn string_stream(self) -> StringStream;
}

impl ResponseExt for Response {
    fn string_stream(self) -> StringStream {
        self.bytes_stream()
            .map_err(|error| AppError::Fatal(error.to_string()))
            .map_ok(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .map_ok(|string| string.trim_start_matches("data:").to_string())
            .boxed()
    }
}
