use crate::{
    agent::{dictionary::DictionaryAgent, prompt::PromptAgent, rag::RagAgent},
    error::Result,
    slm::SlmRequest,
};
use futures::Stream;
use std::pin::Pin;

pub mod dictionary;
pub mod prompt;
pub mod rag;

pub type AgentStream = Pin<Box<dyn Stream<Item = Result<String>>>>;

pub enum Agent {
    Prompt(PromptAgent),
    Rag(RagAgent),
    Dictionary(DictionaryAgent),
}

impl Agent {
    pub async fn exec(&self, request: SlmRequest) -> AgentStream {
        match self {
            Agent::Prompt(agent) => agent.exec(request).await,
            Agent::Rag(agent) => agent.exec(request).await,
            Agent::Dictionary(agent) => agent.exec(request).await,
        }
    }
}
