use crate::proc::Action;
use crate::service::{self};
use crate::types::Result;
use log::{debug, trace};

pub struct MeasureUnitAgent {}

impl<'a> MeasureUnitAgent {
    pub fn new() -> Self {
        trace!("MeasureUnitAgent::new() -> Self");
        Self {}
    }

    pub fn exec(&self, action: &Action<'a>) -> Result<String> {
        trace!("MeasureUnitAgent::exec(&self, action: &Action<'a>) -> Result<String>");
        let prompt = &action.prompt;
        debug!("prompt: {}", prompt);
        service::measure_unit::exec(prompt)
    }
}
