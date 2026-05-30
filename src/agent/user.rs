use log::{debug, trace};

use crate::types::Result;
use crate::service::{self, Property};
use crate::llm::SlmRequest;

pub struct UserProfileAgent {}

impl UserProfileAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn exec(&self, request: &SlmRequest) -> Result<String> {
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
