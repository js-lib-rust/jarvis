use crate::proc::Action;
use crate::service::{self};
use crate::types::Result;
use log::{debug, trace};

pub struct TimeServiceAgent {}

impl<'a> TimeServiceAgent {
    pub(crate) fn new() -> Self {
        trace!("TimeServiceAgent::new() -> Self");
        Self {}
    }

    pub(crate) fn exec(&self, action: &Action<'a>) -> Result<String> {
        trace!("TimeServiceAgent::exec(&self, action: &Action<'a>) -> Result<String>");
        debug!("prompt: {}", &action.prompt);
        service::time_service::exec(&action.prompt)
    }
}
