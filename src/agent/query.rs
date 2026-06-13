use crate::types::{ResponseExt, Result, StringStream};
use crate::llm::LlmRequest;
use log::{debug, trace};

pub(crate) struct QueryAgent {
    http_client: reqwest::Client,
    model_url: &'static str,
}

impl QueryAgent {
    const INSTRUCTION: &'static str = "Execute the following query using the data in context. Generate response in an human readable format as in a natural conversation.";

    pub(crate) fn new() -> Self {
        trace!("QueryAgent::new() -> Self");
        Self {
            http_client: reqwest::Client::new(),
            model_url: "http://jarvis.local/v1/chat/completions",
        }
    }

    pub(crate) async fn exec(&self, system: &str, prompt: &str) -> Result<StringStream> {
        trace!("QueryAgent::exec(&self, system: &str, prompt: &str) -> Result<StringStream>");
        debug!("system: {system}");

        let system = &format!("{}\n\n{}", Self::INSTRUCTION, system);
        let request = LlmRequest::from_messages(system, prompt);
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
