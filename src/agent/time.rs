use crate::service::{self};
use crate::types::Result;
use log::{debug, trace};

pub struct TimeServiceAgent {}

impl TimeServiceAgent {
    pub fn new() -> Self {
        trace!("TimeServiceAgent::new() -> Self");
        Self {}
    }

    pub fn exec(&self, prompt: &str) -> Result<String> {
        trace!("TimeServiceAgent::exec(&self, prompt: &str) -> Result<String>");
        debug!("prompt: {}", prompt);
        service::time::exec(prompt)
    }
}
