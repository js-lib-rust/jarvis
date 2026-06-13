use crate::service::{self};
use crate::types::Result;
use log::{debug, trace};

pub struct MeasurementUnitAgent {}

impl MeasurementUnitAgent {
    pub fn new() -> Self {
        trace!("MeasurementUnitAgent::new() -> Self");
        Self {}
    }

    pub fn exec(&self, prompt: &str) -> Result<String> {
        trace!("MeasurementUnitAgent::exec(&self, prompt: &str) -> Result<String>");
        debug!("prompt: {}", prompt);
        service::measure_unit::exec(prompt)
    }
}
