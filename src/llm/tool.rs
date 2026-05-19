use std::time::Instant;

use crate::error::AppError;
use crate::llm::{SlmClient, SlmRequest};
use crate::types::Result;
use futures::StreamExt;
use log::{debug, trace};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct ToolClient {
    tool_url: String,
    http_client: reqwest::Client,
}

impl ToolClient {
    pub async fn connect(addr: &str) -> Result<Self> {
        let http_client = reqwest::Client::new();
        Ok(Self {
            tool_url: addr.to_string(),
            http_client,
        })
    }

    pub(crate) async fn get_function<T>(&self, prompt: &str, tools: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        trace!(
            "get_function<T>(&self, prompt: &str, tools: &str) -> Result<T> where T: DeserializeOwned"
        );
        let start = Instant::now();
        let request = SlmRequest::with_tools(prompt, tools);
        let result = match self.exec(&request).await {
            Some(tool_code) => serde_json::from_str::<T>(&tool_code)
                .map_err(|error| AppError::Fatal(error.to_string())),
            None => Err(AppError::Fatal(
                "cannot reliable determine the function".to_string(),
            )),
        };
        debug!(
            "Tool function processing time: {} msec.",
            start.elapsed().as_millis()
        );
        result
    }

    async fn exec(&self, request: &SlmRequest) -> Option<String> {
        trace!("exec(&self, request: &SlmRequest) -> Option<String>");
        debug!("request: {:?}", request);

        let slm = SlmClient::for_client(&self.tool_url, &self.http_client);
        let mut stream = slm.exec(request).await;
        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            result.push_str(&chunk.ok()?);
        }
        debug!("result: {}", result);

        // [{"function":{"name":"hera_read_temperature","arguments":{"zone":"living room"}},"confidence":0.9986770153045654,"confidence_min":0.9816742539405824}]
        // {"function":"read_temperature","zone":"living room"}

        #[derive(Deserialize, Debug)]
        struct Function {
            #[serde(rename = "agent")]
            _agent: String,
            name: String,
            arguments: Value,
        }
        #[derive(Deserialize, Debug)]
        struct Response {
            function: Function,
            #[serde(rename = "confidence")]
            _confidence: f32,
            #[serde(rename = "confidence_min")]
            _confidence_min: f32,
        }

        let value: Vec<Response> = serde_json::from_str(&result).ok()?;
        debug!("value: {:?}", value);
        if let Some(response) = value.get(0) {
            let arguments = response
                .function
                .arguments
                .to_string()
                .replace("{", "")
                .replace("}", "");

            let mut response = format!("{{\"function\":\"{}\"", &response.function.name);
            if !arguments.is_empty() {
                response += ",";
                response += &arguments;
            }
            response += "}";
            debug!("response: {}", response);
            Some(response)
        } else {
            None
        }
    }
}
