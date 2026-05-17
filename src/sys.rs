use futures::StreamExt;
use log::{debug, trace};
use serde::Deserialize;
use serde_json::Value;

use crate::slm::{SlmClient, SlmRequest};

pub struct Tool {}

impl Tool {
    pub async fn call(request: SlmRequest) -> Option<String> {
        trace!("sys:Tool::call(request: SlmRequest) -> Option<String>");
        debug!("request: {:?}", request);

        let slm = SlmClient::for_url("http://jarvis.local:1967/");
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
