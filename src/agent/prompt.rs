use crate::agent::AgentStream;
use crate::config;
use crate::slm::SlmClient;
use crate::slm::SlmRequest;
use log::trace;

pub struct PromptAgent{
    system: String,
}

impl PromptAgent {
    const SYSTEM: &'static str = "Question answering agent";

    pub fn new() -> Self {
        let config = config::get_config();
        let system = match &config.prompt_system {
            Some(system) => system,
            None => Self::SYSTEM,
        }.to_string();
        Self {system}
    }

    pub async fn exec(&self, mut request: SlmRequest) -> AgentStream {
        trace!("PromptAgent::exec(&self, mut request: SlmRequest) -> AgentStream");
        request.set_system(&self.system);
        let slm = SlmClient::new();
        slm.exec(request).await
    }
}
