use log::{debug, trace};
use serde::Deserialize;

use crate::{
    sys::Tool,
    types::Result,
    service::weather::{GetCurrentWeather, GetForecast, GetTodayForecast},
    slm::SlmRequest,
};

pub struct WeatherAgent {}

impl WeatherAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execs(&self, mut request: SlmRequest) -> Result<String> {
        trace!("WeatherAgent::execs(&self, mut request: SlmRequest) -> Result<String>");
        debug!("request: {:?}", request);

        request.set_tools("weather");
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
    #[serde(rename = "get_current_weather")]
    GetCurrentWeather(GetCurrentWeather),
    #[serde(rename = "get_forecast")]
    GetForecast(GetForecast),
    #[serde(rename = "get_today_forecast")]
    GetTodayForecast(GetTodayForecast),
}

impl Call {
    async fn exec(&self) -> Result<String> {
        match self {
            Call::GetCurrentWeather(call) => call.exec().await,
            Call::GetForecast(call) => call.exec().await,
            Call::GetTodayForecast(call) => call.exec().await,
        }
    }
}
