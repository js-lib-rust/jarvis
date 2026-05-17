use log::{debug, trace};
use serde::Deserialize;

use crate::{
    service::hera::{
        DescribeDevice, GetDeviceActions, GetHeatingState, ListDevices, ReadHumidity, ReadSensors,
        ReadTemperature, RunDeviceDiagnose, RunDiagnose, RunSystemDiagnose, StartHeating,
        StopHeating,
    },
    slm::SlmRequest,
    sys::Tool,
    types::Result,
};

pub struct HeraAgent {}

impl HeraAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execs(&self, mut request: SlmRequest) -> Result<String> {
        trace!("HeraAgent::execs(&self, request: SlmRequest) -> Result<String>");
        debug!("request: {:?}", request);

        request.set_tools("hera");
        let result = match Tool::call(request).await {
            Some(tool_code) => match serde_json::from_str::<Call>(&tool_code) {
                Ok(call) => match call.exec().await {
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

#[derive(Deserialize, Debug)]
#[serde(tag = "function")]
enum Call {
    #[serde(rename = "list_devices")]
    ListDevices(ListDevices),
    #[serde(rename = "describe_device")]
    DescribeDevice(DescribeDevice),
    #[serde(rename = "get_device_actions")]
    GetDeviceActions(GetDeviceActions),
    #[serde(rename = "read_temperature")]
    ReadTemperature(ReadTemperature),
    #[serde(rename = "read_humidity")]
    ReadHumidity(ReadHumidity),
    #[serde(rename = "read_sensors")]
    ReadSensors(ReadSensors),
    #[serde(rename = "start_heating")]
    StartHeating(StartHeating),
    #[serde(rename = "stop_heating")]
    StopHeating(StopHeating),
    #[serde(rename = "get_heating_state")]
    GetHeatingState(GetHeatingState),
    #[serde(rename = "run_diagnose")]
    RunDiagnose(RunDiagnose),
    #[serde(rename = "run_device_diagnose")]
    RunDeviceDiagnose(RunDeviceDiagnose),
    #[serde(rename = "run_system_diagnose")]
    RunSystemDiagnose(RunSystemDiagnose),
}

impl Call {
    async fn exec(&self) -> Result<String> {
        match self {
            Call::ListDevices(call) => call.exec(),
            Call::DescribeDevice(call) => call.exec().await,
            Call::GetDeviceActions(call) => call.exec().await,
            Call::ReadTemperature(call) => call.exec().await,
            Call::ReadHumidity(call) => call.exec().await,
            Call::ReadSensors(call) => call.exec().await,
            Call::StartHeating(call) => call.exec().await,
            Call::StopHeating(call) => call.exec().await,
            Call::GetHeatingState(call) => call.exec().await,
            Call::RunDiagnose(call) => call.exec().await,
            Call::RunDeviceDiagnose(call) => call.exec().await,
            Call::RunSystemDiagnose(call) => call.exec().await,
        }
    }
}
