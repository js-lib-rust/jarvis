use crate::types::Result;
use crate::{error::AppError, types::StringStream};
use async_stream::stream;
use futures::{StreamExt, executor::block_on};
use log::{debug, error, trace};
use reqwest::Client;
use serde::Serialize;

const SLM_URL: &str = "http://jarvis.local:1964/";

#[derive(Debug, Serialize)]
pub struct SlmRequest {
    prompt: String,
    system: Option<String>,
    tools: Option<String>,
    context: Option<String>,
    use_history: bool,
}

impl SlmRequest {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            system: None,
            tools: None,
            context: None,
            use_history: true,
        }
    }

    pub fn exec(self) -> Result<String> {
        let client = SlmClient::new();
        client.block_exec(self)
    }

    pub fn _set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    pub fn get_prompt(&self) -> &str {
        &self.prompt
    }

    pub fn get_system(&self) -> &Option<String> {
        &self.system
    }

    pub fn add_system(&mut self, system: &str) {
        if let Some(s) = &self.system {
            self.system = Some(format!("{}\n\n{}", s, system));
        } else {
            self.system = Some(system.to_string());
        }
    }

    pub fn set_tools(&mut self, tools: &str) {
        self.tools = Some(tools.to_string());
    }

    pub fn set_use_history(&mut self, use_history: bool) {
        self.use_history = use_history;
    }
}

pub struct SlmClient {
    slm_url: String,
    http_client: Client,
}

impl SlmClient {
    pub fn new() -> Self {
        trace!("SlmClient::new() -> Self");
        Self {
            slm_url: SLM_URL.to_string(),
            http_client: Client::builder().build().unwrap(),
        }
    }

    pub fn for_url(slm_url: &str) -> Self {
        trace!("SlmClient::for_url(slm_url: &str) -> Self");
        let http_client = Client::builder().build().unwrap();
        Self {
            slm_url: slm_url.to_string(),
            http_client,
        }
    }

    pub async fn exec(&self, request: SlmRequest) -> StringStream {
        trace!("SlmClient::exec(&self, request: SlmRequest) -> SlmStream");
        debug!("request: {:?}", request);
        debug!("slm_url: {}", self.slm_url);

        let response = match self
            .http_client
            .post(&self.slm_url)
            .json(&request)
            .send()
            .await
        {
            Ok(response) => Some(response),
            Err(error) => {
                error!("Fail to execute SLM request: {}", error);
                None
            }
        };
        debug!("response: {:?}", response);

        Box::pin(stream! {
            let Some(response) = response else {
                return yield Err(AppError::Fatal("fatal".to_string()));
            };

            if !response.status().is_success() {
                error!("request failed with status: {}", response.status());
                // error!("error response: {}", response.text().await.unwrap());
                yield Err(AppError::Fatal("fatal".to_string()));
            }

            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8(bytes.to_vec()).expect("fail");
                        yield Ok(text);
                    },
                    Err(error) => {
                        error!("Fail to extract bytes from response chunk: {}", error);
                        yield Ok("error".to_string())
                    }
                };
            }
        })
    }

    pub fn block_exec(&self, request: SlmRequest) -> Result<String> {
        trace!("SlmClient::block_exec(&self, request: SlmRequest) -> Result<String>");
        block_on(async {
            let mut stream = self.exec(request).await;
            let mut result = String::new();
            while let Some(chunk) = stream.next().await {
                result.push_str(&chunk.unwrap());
            }
            Ok(result)
        })
    }
}
