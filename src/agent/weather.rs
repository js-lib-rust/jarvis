use crate::{
    llm::ToolClient,
    proc::Action,
    service::weather::{GetCurrentWeather, GetForecast, GetTodayForecast},
    types::Result,
};
use log::trace;
use serde::Deserialize;

pub struct WeatherAgent<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> WeatherAgent<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        Self { tool_client }
    }

    pub async fn exec(&self, action: &Action<'a>) -> Result<String> {
        trace!("exec(&self, action: &Action<'a>) -> Result<String>");
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
    #[serde(rename = "get_current_weather")]
    GetCurrentWeather(GetCurrentWeather),
    #[serde(rename = "get_forecast")]
    GetForecast(GetForecast),
    #[serde(rename = "get_today_forecast")]
    GetTodayForecast(GetTodayForecast),
}

impl Function {
    async fn exec(&self) -> Result<String> {
        match self {
            Function::GetCurrentWeather(call) => call.exec().await,
            Function::GetForecast(call) => call.exec().await,
            Function::GetTodayForecast(call) => call.exec().await,
        }
    }
}
