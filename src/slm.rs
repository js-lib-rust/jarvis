use crate::{agent::AgentStream, config, error::AppError};
use async_stream::stream;
use futures::StreamExt;
use log::{error, trace};
use reqwest::Client;
use serde::Serialize;

const SLM_URL: &str = "http://jarvis.local:1964/";

#[derive(Serialize)]
pub struct SlmRequest {
    prompt: String,
    system: Option<String>,
    profile: Option<String>,
    settings: Option<String>,
    context: Option<String>,
}

impl SlmRequest {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            system: None,
            profile: None,
            settings: None,
            context: None,
        }
    }

    pub fn set_system(&mut self, system: &str) {
        self.system = Some(system.to_string());
    }

    pub fn set_profile(&mut self, profile: &str) {
        self.profile = Some(profile.to_string());
    }

    pub fn set_settings(&mut self, settings: &str) {
        self.settings = Some(settings.to_string());
    }

    pub fn set_context(&mut self, context: &str) {
        self.context = Some(context.to_string());
    }
}

pub struct SlmClient {
    slm_url: String,
    http_client: Client,
}

impl SlmClient {
    pub fn new() -> Self {
        trace!("SlmClient::new() -> Self");
        let config = config::get_config();
        let slm_url = match config.slm_url.as_ref() {
            Some(slm_url) => slm_url,
            None => SLM_URL,
        }
        .to_string();
        let http_client = Client::builder().build().unwrap();
        Self {
            slm_url,
            http_client,
        }
    }

    pub async fn exec(&self, request: SlmRequest) -> AgentStream {
        trace!("exec(&self, request: SlmRequest) -> SlmStream");

        let response = self
            .http_client
            .post(&self.slm_url)
            .json(&request)
            .send()
            .await
            .unwrap();

        Box::pin(stream! {
            if !response.status().is_success() {
                error!("request failed with status: {}", response.status());
                //error!("error response: {}", response.text().await.unwrap());
                yield Err(AppError::Fatal("fatal".to_string()));
            }

            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let bytes = chunk.expect("fail");
                let text = String::from_utf8(bytes.to_vec()).expect("fail");
                yield Ok(text);
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        slm::{SlmClient, SlmRequest},
    };
    use futures::StreamExt;
    use tokio::io::{self, AsyncWriteExt};

    #[tokio::test]
    async fn prompt() {
        let prompt = "how to eprint to stdout on rust";
        let request = SlmRequest::new(prompt);

        let slm_client = SlmClient::new();
        let mut stream = slm_client.exec(request).await;
        while let Some(chunk) = stream.next().await {
            print!("{}", chunk.unwrap());
            io::stdout().flush().await.unwrap();
        }
        println!();
    }

    #[tokio::test]
    async fn query() {
        let context = "context";
        let prompt = "please list all acronyms";
        let mut request = SlmRequest::new(prompt);
        request.set_context(context);

        let slm_client = SlmClient::new();
        let mut stream = slm_client.exec(request).await;
        while let Some(chunk) = stream.next().await {
            print!("{}", chunk.unwrap());
            io::stdout().flush().await.unwrap();
        }
        println!();
    }
}
