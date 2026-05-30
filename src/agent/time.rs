use log::{debug, trace};

use crate::types::Result;
use crate::service::{self};
use crate::llm::SlmRequest;

pub struct TimeServiceAgent {}

impl TimeServiceAgent {
    pub fn new() -> Self {
        trace!("TimeServiceAgent::new() -> Self");
        Self {}
    }

    pub fn exec(&self, request: &SlmRequest) -> Result<String> {
        trace!("TimeServiceAgent::exec(&self, request: SlmRequest) -> Result<String>");
        let prompt = request.get_prompt().to_string();
        debug!("prompt: {prompt}");
        service::time::exec(&prompt)
    }
}
