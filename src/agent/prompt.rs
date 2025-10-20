use crate::agent::AgentStream;
use crate::slm::SlmClient;
use crate::slm::SlmRequest;
use log::trace;

pub struct PromptAgent;

impl PromptAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn exec(&self, request: SlmRequest) -> AgentStream {
        trace!("PromptAgent::exec(request: SlmRequest) -> SlmStream");
        let slm = SlmClient::new();
        slm.exec(request).await
    }
}
