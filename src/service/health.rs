use crate::types::Result;
use bson::Bson;
use chrono::{Local, TimeZone, Utc};
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, trace};
use mongodb::{
    Client, Collection,
    bson::{DateTime, doc},
    options::FindOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

lazy_static! {
    static ref UNITS: HashMap<&'static str, &'static str> = {
        let mut m: HashMap<&'static str, &'static str> = HashMap::new();
        m.insert("systolic_pressure", "mmHg");
        m.insert("diastolic_pressure", "mmHg");
        m.insert("pulse_pressure", "mmHg");
        m.insert("heart_rate", "bpm");
        m.insert("glucose_level", "mg/dL");
        m.insert("body_temperature", "°C");
        m.insert("body_weight", "kg");
        m.insert("body_mass_index", "kg/m²");
        m
    };
}

fn default_timestamp() -> DateTime {
    DateTime::now()
}

fn default_option_f32() -> Option<f32> {
    None
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SaveBlood {
    #[serde(default = "default_timestamp")]
    timestamp: DateTime,

    person: String,
    systole: u32,
    diastole: u32,
    pulse: u32,
}

impl SaveBlood {
    pub async fn exec(&self) -> Result<String> {
        trace!("SaveBlood::exec(&self) -> Result<String>");
        debug!(
            "person: {}, systole: {}, diastole: {}, pulse: {}",
            self.person, self.systole, self.diastole, self.pulse
        );

        let mut result = String::new();
        let archive = Archive::new().await?;
        for (key, value) in [
            ("systolic_pressure", Bson::from(self.systole)),
            ("diastolic_pressure", Bson::from(self.diastole)),
            ("pulse_pressure", Bson::from(self.systole - self.diastole)),
            ("heart_rate", Bson::from(self.pulse)),
        ] {
            let measurement = archive.save_measurement(&self.person, key, value).await?;
            result.push_str(&serde_json::to_string(&measurement)?);
            result.push('\n');
        }

        Ok(result)
    }
}

#[derive(Deserialize, Serialize)]
pub struct SaveTemperature {
    #[serde(default = "default_timestamp")]
    timestamp: DateTime,

    person: String,
    temperature: f32,
}

impl SaveTemperature {
    pub async fn exec(&self) -> Result<String> {
        trace!("SaveTemperature::exec(&self) -> Result<String>");
        debug!("person: {}, temperature: {}", self.person, self.temperature);

        let archive = Archive::new().await?;
        let measurement = archive
            .save_measurement(
                &self.person,
                "body_temperature",
                Bson::from(self.temperature),
            )
            .await?;

        Ok(serde_json::to_string(&measurement)?)
    }
}

#[derive(Deserialize, Serialize)]
pub struct SaveWeight {
    #[serde(default = "default_timestamp")]
    timestamp: DateTime,

    person: String,
    height: f32,
    weight: f32,
    #[serde(default = "default_option_f32")]
    body_mass_index: Option<f32>,
}

impl SaveWeight {
    pub async fn exec(&mut self) -> Result<String> {
        trace!("SaveWeight::exec(&mut self) -> Result<String>");
        debug!(
            "person: {}, height: {}, weight: {}",
            self.person, self.height, self.weight
        );

        let height = if self.height < 2.5 {
            self.height
        } else {
            self.height / 100.0
        };
        let bmi = self.weight / (height * height);
        self.body_mass_index = Some(bmi);

        let mut result = String::new();
        let archive = Archive::new().await?;
        for (key, value) in [
            ("body_weight", Bson::from(&self.weight)),
            ("body_mass_index", Bson::from(&bmi)),
        ] {
            let measurement = archive.save_measurement(&self.person, key, value).await?;
            result.push_str(&serde_json::to_string(&measurement)?);
            result.push('\n');
        }

        Ok(result)
    }
}

#[derive(Deserialize, Serialize)]
pub struct SaveGlucose {
    #[serde(default = "default_timestamp")]
    timestamp: DateTime,

    person: String,
    glucose: f32,
}

impl SaveGlucose {
    pub async fn exec(&self) -> Result<String> {
        trace!("SaveGlucose::exec(&self) -> Result<String>");
        debug!("person: {}, glucose: {}", self.person, self.glucose);

        let archive = Archive::new().await?;
        let measurement = archive
            .save_measurement(&self.person, "glucose_level", Bson::from(self.glucose))
            .await?;

        Ok(serde_json::to_string(&measurement)?)
    }
}

#[derive(Deserialize)]
pub struct ReadMeasurements {
    person: String,
    date: String,
}

impl ReadMeasurements {
    pub async fn exec(&self) -> Result<String> {
        trace!("ReadMeasurements::exec(&self) -> Result<String>");
        debug!("person: {}, date: {}", self.person, self.date);

        let archive = Archive::new().await?;
        let measurements = archive.read_measurements(&self.person, &self.date).await?;

        let result = measurements
            .iter()
            .filter_map(|m| serde_json::to_string(m).ok())
            .collect::<Vec<String>>()
            .join("\n");
        debug!("result: {result}");
        Ok(result)
    }
}

pub struct Archive {
    collection: Collection<Record>,
}

impl Archive {
    const SERVER: &'static str = "mongodb://localhost:27017";
    const DATABASE: &'static str = "jarvis";
    const COLLECTION: &'static str = "health";

    pub async fn new() -> Result<Self> {
        trace!("Archive::new() -> Result<Self>");
        let client = Client::with_uri_str(Self::SERVER).await?;
        let database = client.database(Archive::DATABASE);
        let collection = database.collection::<Record>(Archive::COLLECTION);

        Ok(Self { collection })
    }

    async fn save_measurement(
        &self,
        person: &str,
        measurement: &str,
        value: Bson,
    ) -> Result<Measurement> {
        let units = match UNITS.get(measurement) {
            Some(units) => Some(units.to_string()),
            None => None,
        };
        let record = Record {
            timestamp: DateTime::now(),
            date: Local::now().format("%Y-%m-%d").to_string(),
            person: person.to_string(),
            measurement: measurement.to_string(),
            value: value,
            units: units,
        };
        let _ = self.collection.insert_one(&record, None).await?;
        Ok(Measurement::new(record))
    }

    async fn read_measurements(&self, person: &str, date: &str) -> Result<Vec<Measurement>> {
        trace!(
            "Archive::read_measurements(&self, person: &str, date: &str) -> Result<HashMap<String, String>>"
        );
        let filter = doc! {"$and":[{"person":person},{"date":date}]};
        let options = FindOptions::builder().build();

        let mut measurements = Vec::<Measurement>::new();
        let mut cursor = self.collection.find(filter, options).await?;
        while let Some(result) = cursor.next().await {
            match result {
                Ok(record) => {
                    debug!("record: {record:?}");
                    measurements.push(Measurement::new(record));
                }
                Err(e) => eprintln!("Error loading document: {}", e),
            }
        }
        Ok(measurements)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Record {
    timestamp: DateTime,
    date: String,
    person: String,
    measurement: String,
    value: Bson,
    units: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Measurement {
    timestamp: String,
    person: String,
    measurement: String,
    value: Bson,
    units: Option<String>,
}

impl Measurement {
    fn new(record: Record) -> Self {
        Self {
            timestamp: Self::bson_datetime_local(record.timestamp),
            person: record.person,
            measurement: record.measurement,
            value: record.value,
            units: record.units,
        }
    }

    fn bson_datetime_local(bson_dt: DateTime) -> String {
        let utc_dt = Utc
            .timestamp_opt(bson_dt.timestamp_millis() / 1000, 0)
            .single()
            .unwrap_or(Utc.timestamp_opt(0, 0).unwrap());
        let local_dt = utc_dt.with_timezone(&Local);
        local_dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}
