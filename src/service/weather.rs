use log::{debug, trace};
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp;

use crate::error::AppError;
use crate::types::Result;

#[derive(Deserialize, Debug)]
pub struct GetCurrentWeather {
    locality: String,
}

impl GetCurrentWeather {
    pub async fn exec(&self) -> Result<String> {
        trace!("GetCurrentWeather::exec(&self) -> Result<String>");
        debug!("locality: {}", self.locality);

        #[derive(Serialize)]
        struct CurrentWeather<'a> {
            locality: &'a str,
            condition: &'a str,
            temperature: f32,
            temperature_feeling: &'a str,
            temperature_units: &'a str,
            wind: f32,
            wind_feeling: &'a str,
            wind_units: &'a str,
            precipitation_chance: f32,
            precipitation_chance_feeling: &'a str,
            precipitation_chance_units: &'a str,
        }

        let weather = get_current_weather(47.180530747905784, 27.488742039539478).await?;
        let weather = CurrentWeather {
            locality: &self.locality,
            condition: &weather.current.weather_code,
            temperature: weather.current.temperature_2m,
            temperature_feeling: get_temperature_feeling(weather.current.temperature_2m),
            temperature_units: &weather.current_units.temperature_2m,
            wind: weather.current.wind_speed_10m,
            wind_feeling: get_wind_feeling(weather.current.wind_speed_10m),
            wind_units: &weather.current_units.wind_speed_10m,
            precipitation_chance: weather.current.precipitation,
            precipitation_chance_feeling: get_precipitation_feeling(weather.current.precipitation),
            precipitation_chance_units: "%",
        };
        Ok(serde_json::to_string(&weather)?)
    }
}

#[derive(Deserialize, Debug)]
pub struct GetForecast {
    locality: String,
    days: u8,
}

impl GetForecast {
    pub async fn exec(&self) -> Result<String> {
        trace!("GetForecast::exec(&self) -> Result<String>");
        debug!("locality: {}, days: {}", self.locality, self.days);

        #[derive(Serialize)]
        struct DayForecast<'a> {
            date: &'a str,
            condition: &'a str,
            temperature_max: f32,
            temperature_min: f32,
            wind: f32,
            precipitation: Option<i32>,
        }

        let forecast =
            get_weather_forecast(47.180530747905784, 27.488742039539478, self.days, false).await?;

        let mut result = String::new();
        for i in 0..forecast.daily.time.len() {
            let day_forecast = DayForecast {
                date: &forecast.daily.time[i],
                condition: &forecast.daily.weather_code[i],
                temperature_max: forecast.daily.temperature_2m_max[i],
                temperature_min: forecast.daily.temperature_2m_min[i],
                wind: forecast.daily.wind_speed_10m_max[i],
                precipitation: forecast.daily.precipitation_probability_max[i],
            };
            result.push_str(&serde_json::to_string(&day_forecast)?);
            result.push('\n');
        }
        Ok(result)
    }
}

#[derive(Deserialize, Debug)]
pub struct GetTodayForecast {
    locality: String,
}

impl GetTodayForecast {
    pub async fn exec(&self) -> Result<String> {
        trace!("GetTodayForecast::exec(&self) -> Result<String>");
        debug!("locality: {}", self.locality);

        #[derive(Serialize)]
        struct HourForecast<'a> {
            time: &'a str,
            condition: &'a str,
            temperature: f32,
            wind: f32,
            precipitation: Option<i32>,
        }

        let forecast =
            get_weather_forecast(47.180530747905784, 27.488742039539478, 1, true).await?;

        let Some(hourly) = forecast.hourly else {
            return Ok(String::from("missing hourly forecast from server"));
        };
        let mut result = String::new();
        for i in 0..hourly.time.len() {
            let hour_forecast = HourForecast {
                time: &hourly.time[i],
                condition: &hourly.weather_code[i],
                temperature: hourly.temperature_2m[i],
                wind: hourly.wind_speed_10m[i],
                precipitation: hourly.precipitation_probability[i],
            };
            result.push_str(&serde_json::to_string(&hour_forecast)?);
            result.push('\n');
        }
        Ok(result)
    }
}

// --------------------------------------------------------
// OPEN METEO

#[derive(Debug, Serialize, Deserialize)]
struct WeatherResponse {
    latitude: f32,
    longitude: f32,
    generationtime_ms: f32,
    utc_offset_seconds: i32,
    timezone: String,
    timezone_abbreviation: String,
    elevation: f32,
    current: CurrentWeather,
    current_units: CurrentUnits,
}

#[derive(Debug, Serialize, Deserialize)]
struct CurrentWeather {
    #[serde(rename(serialize = "weather"))]
    #[serde(deserialize_with = "weather_code_description")]
    weather_code: String,
    temperature_2m: f32,
    wind_speed_10m: f32,
    precipitation: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CurrentUnits {
    weather_code: String,
    temperature_2m: String,
    wind_speed_10m: String,
    precipitation: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForecastResponse {
    latitude: f32,
    longitude: f32,
    elevation: f32,
    generationtime_ms: f32,
    utc_offset_seconds: i32,
    timezone: String,
    timezone_abbreviation: String,
    hourly: Option<HourlyForecast>,
    hourly_units: Option<HourlyUnits>,
    daily: DailyForecast,
    daily_units: DailyUnits,
}

#[derive(Debug, Serialize, Deserialize)]
struct HourlyForecast {
    time: Vec<String>,
    #[serde(deserialize_with = "weather_codes_description")]
    weather_code: Vec<String>,
    temperature_2m: Vec<f32>,
    wind_speed_10m: Vec<f32>,
    precipitation_probability: Vec<Option<i32>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HourlyUnits {
    time: String,
    weather_code: String,
    temperature_2m: String,
    wind_speed_10m: String,
    precipitation_probability: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DailyForecast {
    time: Vec<String>,
    #[serde(deserialize_with = "weather_codes_description")]
    weather_code: Vec<String>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    wind_speed_10m_max: Vec<f32>,
    precipitation_probability_max: Vec<Option<i32>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DailyUnits {
    time: String,
    weather_code: String,
    temperature_2m_max: String,
    temperature_2m_min: String,
    wind_speed_10m_max: String,
    precipitation_probability_max: String,
}

async fn get_current_weather(lat: f32, lon: f32) -> Result<WeatherResponse> {
    trace!("get_current_weather(lat: f32, lon: f32) -> Result<WeatherResponse>");
    debug!("lat: {lat}, lon: {lon}");

    let url = "https://api.open-meteo.com/v1/forecast";

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .query(&[
            ("latitude", lat.to_string().as_str()),
            ("longitude", lon.to_string().as_str()),
            (
                "current",
                "weather_code,temperature_2m,wind_speed_10m,precipitation",
            ),
            ("temperature_unit", "celsius"),
            ("wind_speed_unit", "kmh"),
            ("timezone", "auto"),
        ])
        .send()
        .await?;
    Ok(response.json().await?)
}

async fn get_weather_forecast(
    lat: f32,
    lon: f32,
    days: u8,
    hourly: bool,
) -> Result<ForecastResponse> {
    trace!(
        "get_weather_forecast(lat: f32, lon: f32, days: u8, hourly: bool) -> Result<ForecastResponse>"
    );
    debug!("lat: {lat}, lon: {lon}, days: {days}");

    let url = "https://api.open-meteo.com/v1/forecast";
    let lat = lat.to_string();
    let lon = lon.to_string();
    // server returns today as the first day of the forecast period; we need to skip it for days forecast
    let days = cmp::min(if hourly { days } else { days + 1 }, 16).to_string();

    let mut query = vec![
        ("latitude", lat.as_str()),
        ("longitude", lon.as_str()),
        ("forecast_days", days.as_str()),
        (
            "daily",
            "weather_code,temperature_2m_max,temperature_2m_min,wind_speed_10m_max,precipitation_probability_max",
        ),
        ("temperature_unit", "celsius"),
        ("wind_speed_unit", "kmh"),
        ("precipitation_unit", "mm"),
        ("timezone", "auto"),
    ];
    if hourly {
        query.push((
            "hourly",
            "weather_code,temperature_2m,wind_speed_10m,precipitation_probability",
        ));
    }

    let client = reqwest::Client::new();
    let response = client.get(url).query(&query).send().await?;
    if !response.status().is_success() {
        return Err(AppError::Fatal(format!("API error: {}", response.status())));
    }

    let mut forecast: ForecastResponse = response.json().await?;
    if !hourly {
        forecast.daily.time.remove(0);
        forecast.daily.weather_code.remove(0);
        forecast.daily.temperature_2m_max.remove(0);
        forecast.daily.temperature_2m_min.remove(0);
        forecast.daily.wind_speed_10m_max.remove(0);
        forecast.daily.precipitation_probability_max.remove(0);
    }
    Ok(forecast)
}

fn weather_code_description<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let code: i32 = serde::Deserialize::deserialize(deserializer)?;
    Ok(get_weather_code_description(code).to_string())
}

fn weather_codes_description<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let codes: Vec<i32> = serde::Deserialize::deserialize(deserializer)?;
    Ok(codes
        .into_iter()
        .map(|code| get_weather_code_description(code).to_string())
        .collect::<Vec<String>>())
}

fn get_weather_code_description(code: i32) -> &'static str {
    match code {
        0 => "clear sky",
        1 => "mainly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 => "fog",
        48 => "depositing rime fog",
        51 => "light drizzle",
        53 => "moderate drizzle",
        55 => "dense drizzle",
        56 => "light freezing drizzle",
        57 => "dense freezing drizzle",
        61 => "slight rain",
        63 => "moderate rain",
        65 => "heavy rain",
        66 => "light freezing rain",
        67 => "heavy freezing rain",
        71 => "slight snow fall",
        73 => "moderate snow fall",
        75 => "heavy snow fall",
        77 => "snow grains",
        80 => "slight rain showers",
        81 => "moderate rain showers",
        82 => "violent rain showers",
        85 => "slight snow showers",
        86 => "heavy snow showers",
        95 => "thunderstorm",
        96 => "thunderstorm with slight hail",
        99 => "thunderstorm with heavy hail",
        _ => "unknown weather code",
    }
}

fn get_temperature_feeling(temperature: f32) -> &'static str {
    match temperature {
        t if t <= -15.0 => "bitterly freezing",
        t if t > -15.0 && t <= -5.0 => "extremely cold",
        t if t > -5.0 && t <= 0.0 => "freezing",
        t if t > 0.0 && t <= 5.0 => "very cold",
        t if t > 5.0 && t <= 10.0 => "cold",
        t if t > 10.0 && t <= 13.0 => "chilly",
        t if t > 13.0 && t <= 16.0 => "cool",
        t if t > 16.0 && t <= 19.0 => "slightly cool",
        t if t > 19.0 && t <= 22.0 => "pleasant",
        t if t > 22.0 && t <= 25.0 => "mild",
        t if t > 25.0 && t <= 28.0 => "warm",
        t if t > 28.0 && t <= 32.0 => "hot",
        t if t > 32.0 && t <= 36.0 => "very hot",
        t if t > 36.0 && t <= 40.0 => "extremely hot",
        _ => "dangerously scorching",
    }
}

fn get_precipitation_feeling(probability: f32) -> &'static str {
    match probability {
        p if p < 0.0 => "invalid probability",
        p if p == 0.0 => "no precipitation expected",
        p if p > 0.0 && p <= 10.0 => "very slight chance",
        p if p > 10.0 && p <= 20.0 => "slight chance",
        p if p > 20.0 && p <= 30.0 => "small chance",
        p if p > 30.0 && p <= 40.0 => "possible",
        p if p > 40.0 && p <= 50.0 => "moderate chance",
        p if p > 50.0 && p <= 60.0 => "likely",
        p if p > 60.0 && p <= 70.0 => "very likely",
        p if p > 70.0 && p <= 80.0 => "highly likely",
        p if p > 80.0 && p <= 90.0 => "almost certain",
        p if p > 90.0 && p < 100.0 => "certain",
        p if p == 100.0 => "definite",
        _ => "invalid probability",
    }
}

fn get_wind_feeling(wind_speed: f32) -> &'static str {
    match wind_speed {
        w if w < 1.0 => "calm",
        w if w < 6.0 => "light air",
        w if w < 12.0 => "light breeze",
        w if w < 20.0 => "gentle breeze",
        w if w < 29.0 => "moderate breeze",
        w if w < 39.0 => "fresh breeze",
        w if w < 50.0 => "strong breeze",
        w if w < 62.0 => "near gale",
        w if w < 75.0 => "gale",
        w if w < 89.0 => "strong gale",
        w if w < 103.0 => "storm",
        w if w < 118.0 => "violent storm",
        w if w < 200.0 => "hurricane",
        _ => "invalid wind speed",
    }
}
