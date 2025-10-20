use crate::{agent::AgentStream, error::AppError};
use async_stream::stream;
use futures::StreamExt;
use log::{error, trace};
use reqwest::Client;
use serde::Serialize;

const SLM_URL: &str = "http://jarvis.local:1964/";

#[derive(Serialize)]
pub struct SlmRequest {
    template: Option<String>,
    system: Option<String>,
    context: Option<String>,
    prompt: String,
}

impl SlmRequest {
    pub fn builder() -> SlmRequestBuilder {
        SlmRequestBuilder::new()
    }

    pub fn set_system(&mut self, system: String) {
        self.system = Some(system);
    }

    pub fn get_system(&self) -> Option<String> {
        self.system.clone()
    }
}

pub struct SlmRequestBuilder {
    template: Option<String>,
    system: Option<String>,
    context: Option<String>,
    prompt: Option<String>,
}

impl SlmRequestBuilder {
    pub fn new() -> Self {
        Self {
            template: None,
            system: None,
            context: None,
            prompt: None,
        }
    }

    pub fn template(&mut self, template: String) -> &mut Self {
        self.template = Some(template);
        self
    }

    pub fn system(&mut self, system: String) -> &mut Self {
        self.system = Some(system);
        self
    }

    pub fn context(&mut self, context: String) -> &mut Self {
        self.context = Some(context);
        self
    }

    pub fn prompt(&mut self, prompt: String) -> &mut Self {
        self.prompt = Some(prompt);
        self
    }

    pub fn build(&self) -> SlmRequest {
        SlmRequest {
            template: self.template.clone(),
            system: self.system.clone(),
            context: self.context.clone(),
            prompt: self.prompt.clone().unwrap(),
        }
    }
}

pub struct SlmClient {
    http_client: Client,
}

impl SlmClient {
    pub fn new() -> Self {
        trace!("SlmClient::new() -> Self");
        let http_client = Client::builder().build().unwrap();
        Self { http_client }
    }

    pub async fn exec(&self, request: SlmRequest) -> AgentStream {
        trace!("exec(&self, request: SlmRequest) -> SlmStream");

        let response = self
            .http_client
            .post(SLM_URL)
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
    use crate::slm::{SlmClient, SlmRequest};
    use futures::StreamExt;
    use tokio::io::{self, AsyncWriteExt};

    #[tokio::test]
    async fn prompt() {
        let prompt = String::from("how to eprint to stdout on rust");
        let request = SlmRequest::builder().prompt(prompt).build();

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
        let context = String::from("context");
        let prompt = String::from("please list all acronyms");
        let request = SlmRequest::builder()
            .context(context)
            .prompt(prompt)
            .build();

        let slm_client = SlmClient::new();
        let mut stream = slm_client.exec(request).await;
        while let Some(chunk) = stream.next().await {
            print!("{}", chunk.unwrap());
            io::stdout().flush().await.unwrap();
        }
        println!();
    }
}
