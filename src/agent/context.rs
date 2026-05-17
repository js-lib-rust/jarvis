use log::{debug, trace};

use crate::types::Result;
use crate::service::{self, Property};
use crate::slm::SlmRequest;

pub struct TimeServiceAgent {}

impl TimeServiceAgent {
    pub fn new() -> Self {
        trace!("TimeServiceAgent::new() -> Self");
        Self {}
    }

    pub fn execs(&self, request: SlmRequest) -> Result<String> {
        trace!("TimeServiceAgent::exec(&self, request: SlmRequest) -> Result<String>");
        let prompt = request.get_prompt().to_string();
        debug!("prompt: {prompt}");
        service::time::exec(&prompt)
    }
}

pub struct UserProfileAgent {}

impl UserProfileAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execs(&self, request: SlmRequest) -> Result<String> {
        trace!("UserProfileAgent::exec(&self, request: SlmRequest) -> Result<String>");
        let prompt = request.get_prompt().to_string();
        debug!("prompt: {prompt}");

        if prompt == "Get my username." {
            return Ok(Property::value("username", &self.username()));
        }
        service::user::exec(&prompt)
    }

    fn username(&self) -> String {
        String::from("Rotaru Iulian")
    }
}

pub struct MeasurementUnitAgent {}

impl MeasurementUnitAgent {
    pub fn new() -> Self {
        trace!("MeasurementUnitAgent::new() -> Self");
        Self {}
    }

    pub fn execs(&self, request: SlmRequest) -> Result<String> {
        trace!("MeasurementUnitAgent::exec(&self, request: SlmRequest) -> Result<String>");
        let prompt = request.get_prompt().to_string();
        debug!("prompt: {prompt}");
        service::measure_unit::exec(&prompt)
    }
}
