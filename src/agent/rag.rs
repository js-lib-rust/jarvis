use crate::agent::AgentStream;
use crate::config;
use crate::slm::SlmClient;
use crate::slm::SlmRequest;
use log::trace;

pub struct RagAgent{
    system: String,
}

impl RagAgent {
    const SYSTEM: &'static str = "Contextual question answering agent that always uses only \
    the provided context and formats answers in Markdown. If requested information is missing \
    from context, respond with 'I do not know'.";

    pub fn new() -> Self {
        let config = config::get_config();
        let system = match &config.rag_system {
            Some(system) => system,
            None => Self::SYSTEM,
        }.to_string();
        Self {system}
    }

    pub async fn exec(&self, request: &mut SlmRequest) -> AgentStream {
        trace!("RagAgent::exec(&self, mut request: SlmRequest) -> AgentStream");
        request.set_system(&self.system);
        let slm = SlmClient::new();
        slm.exec(request).await
    }
}
