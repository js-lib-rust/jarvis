use log::{debug, trace};
use serde::Deserialize;

use crate::service::health::{
    ReadMeasurements, SaveBlood, SaveGlucose, SaveTemperature, SaveWeight,
};
use crate::types::Result;
use crate::{slm::SlmRequest, sys::Tool};

pub struct HealthAgent {}

impl HealthAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execs(&self, mut request: SlmRequest) -> Result<String> {
        trace!("HealthAgent::execs(&self, mut request: SlmRequest) -> Result<String>");
        debug!("request: {:?}", request);

        request.set_tools("health");
        let result = match Tool::call(request).await {
            Some(tool_code) => match serde_json::from_str::<Call>(&tool_code) {
                Ok(mut call) => match call.exec().await {
                    Ok(result) => result,
                    Err(error) => error.to_string(),
                },
                Err(_) => tool_code,
            },
            None => String::from("cannot reliable determine the function"),
        };
        debug!("tool result: {result}");
        Ok(result)
    }
}

#[derive(Deserialize)]
#[serde(tag = "function")]
enum Call {
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

impl Call {
    async fn exec(&mut self) -> Result<String> {
        match self {
            Call::SaveBlood(call) => call.exec().await,
            Call::SaveTemperature(call) => call.exec().await,
            Call::SaveWeight(call) => call.exec().await,
            Call::SaveGlucose(call) => call.exec().await,
            Call::ReadMeasurements(call) => call.exec().await,
        }
    }
}
