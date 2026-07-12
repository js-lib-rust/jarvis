use crate::{
    llm::ToolClient,
    proc::Action,
    service::home_automation::{
        DescribeDevice, GetDeviceActions, GetHeatingState, ListDevices, ReadHumidity, ReadSensors,
        ReadTemperature, RunDeviceDiagnose, RunDiagnose, RunSystemDiagnose, StartHeating,
        StopHeating,
    },
    types::Result,
};
use log::trace;
use serde::Deserialize;

pub struct HomeAutomationAgent<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> HomeAutomationAgent<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        Self { tool_client }
    }

    pub async fn exec(&self, action: &Action<'a>) -> Result<String> {
        trace!("HomeAutomationAgent::exec(&self, action: &Action<'a>) -> Result<String>");
        let function: Function = self
            .tool_client
            .get_function(&action.prompt, action.agent)
            .await?;
        function.exec().await
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "function")]
enum Function {
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

impl Function {
    async fn exec(&self) -> Result<String> {
        match self {
            Function::ListDevices(call) => call.exec(),
            Function::DescribeDevice(call) => call.exec().await,
            Function::GetDeviceActions(call) => call.exec().await,
            Function::ReadTemperature(call) => call.exec().await,
            Function::ReadHumidity(call) => call.exec().await,
            Function::ReadSensors(call) => call.exec().await,
            Function::StartHeating(call) => call.exec().await,
            Function::StopHeating(call) => call.exec().await,
            Function::GetHeatingState(call) => call.exec().await,
            Function::RunDiagnose(call) => call.exec().await,
            Function::RunDeviceDiagnose(call) => call.exec().await,
            Function::RunSystemDiagnose(call) => call.exec().await,
        }
    }
}
