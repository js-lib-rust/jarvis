use log::{debug, trace};

use crate::types::Result;
use crate::service::{self};
use crate::llm::SlmRequest;

pub struct MeasurementUnitAgent {}

impl MeasurementUnitAgent {
    pub fn new() -> Self {
        trace!("MeasurementUnitAgent::new() -> Self");
        Self {}
    }

    pub fn exec(&self, request: &SlmRequest) -> Result<String> {
        trace!("MeasurementUnitAgent::exec(&self, request: SlmRequest) -> Result<String>");
        let prompt = request.get_prompt().to_string();
        debug!("prompt: {prompt}");
        service::measure_unit::exec(&prompt)
    }
}
