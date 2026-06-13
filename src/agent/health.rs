use crate::llm::ToolClient;
use crate::service::health::{
    ReadMeasurements, SaveBlood, SaveGlucose, SaveTemperature, SaveWeight,
};
use crate::types::Result;
use log::trace;
use serde::Deserialize;

pub struct HealthAgent<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> HealthAgent<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        Self { tool_client }
    }

    pub async fn exec(&self, prompt: &str) -> Result<String> {
        trace!("HealthAgent::exec(&self, prompt: &str) -> Result<String>");
        let mut function: Function = self.tool_client.get_function(prompt, "health").await?;
        function.exec().await
    }
}

#[derive(Deserialize)]
#[serde(tag = "function")]
enum Function {
    #[serde(rename = "save_blood_measurement")]
    SaveBlood(SaveBlood),
    #[serde(rename = "save_temperature")]
    SaveTemperature(SaveTemperature),
    #[serde(rename = "save_weight")]
    SaveWeight(SaveWeight),
    #[serde(rename = "save_glucose")]
    SaveGlucose(SaveGlucose),
    #[serde(rename = "read_medical_records")]
    ReadMeasurements(ReadMeasurements),
}

impl Function {
    async fn exec(&mut self) -> Result<String> {
        match self {
            Function::SaveBlood(call) => call.exec().await,
            Function::SaveTemperature(call) => call.exec().await,
            Function::SaveWeight(call) => call.exec().await,
            Function::SaveGlucose(call) => call.exec().await,
            Function::ReadMeasurements(call) => call.exec().await,
        }
    }
}
