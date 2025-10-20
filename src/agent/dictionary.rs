use crate::agent::AgentStream;
use crate::slm::SlmRequest;
use async_stream::stream;
use log::trace;

pub struct DictionaryAgent;

impl DictionaryAgent {
    pub const ID: &'static str = "DictionaryAgent";

    pub fn new() -> Self {
        Self {}
    }

    pub async fn exec(&self, request: SlmRequest) -> AgentStream {
        trace!("DictionaryAgent::exec(&self, request: SlmRequest) -> AgentStream");

        Box::pin(stream! {
            yield Ok(String::from(request.get_system().unwrap()))
        })
    }
}
