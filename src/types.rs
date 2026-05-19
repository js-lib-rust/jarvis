use crate::error::AppError;
use crate::llm::{RouterClient, ToolClient};
use axum::body::Bytes;
use axum::response::sse::Event;
use futures::StreamExt;
use futures::{TryStreamExt, stream::Stream};
use reqwest::Response;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct AppState {
    pub(crate) router_client: RouterClient,
    pub(crate) tool_client: ToolClient,
    pub(crate) http_client: reqwest::Client,
    pub(crate) model_url: String,
}

impl AppState {
    pub(crate) async fn create(
        router_addr: &str,
        tool_url: &str,
        model_url: &str,
    ) -> Result<Arc<AppState>> {
        let router_client = RouterClient::connect(router_addr).await?;
        let tool_client = ToolClient::connect(tool_url).await?;
        let http_client = reqwest::Client::new();
        let model_url = model_url.to_string();
        Ok(Arc::new(AppState {
            router_client,
            tool_client,
            http_client,
            model_url,
        }))
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
