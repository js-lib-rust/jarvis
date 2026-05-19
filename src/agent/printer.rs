use crate::llm::SlmRequest;
use crate::types::{ResponseExt, Result, StringStream};
use crate::{error::AppError, llm::LlmRequest};
use log::{debug, trace};

pub struct PrinterAgent {
    http_client: reqwest::Client,
    model_url: &'static str,
}

impl PrinterAgent {
    const INSTRUCTION: &'static str = "Print a human readable response based on user prompt and provided context data. Follow formatting instructions from user prompt.";

    pub fn new() -> Self {
        trace!("PrinterAgent::new() -> Self");
        Self {
            http_client: reqwest::Client::new(),
            model_url: "http://jarvis.local/v1/chat/completions",
        }
    }

    pub async fn exec(&self, request: SlmRequest) -> Result<StringStream> {
        trace!("PrinterAgent::execs(&self, request: SlmRequest) -> StringStream");
        let Some(system) = request.get_system() else {
            return Err(AppError::Fatal("missing system".to_string()));
        };
        debug!("system: {system}");

        let system = &format!("{}\n\n{}", Self::INSTRUCTION, system);
        let request = LlmRequest::from_messages(system, request.get_prompt());
        debug!("request: {:?}", request);

        Ok(self
            .http_client
            .post(self.model_url)
            .json(&request)
            .send()
            .await?
            .string_stream())
    }
}
