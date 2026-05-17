use crate::types::Result;
use bson::DateTime;
use chrono::{Local, TimeZone, Utc};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) mod health;
pub(crate) mod measure_unit;
pub(crate) mod time;
pub(crate) mod user;
pub(crate) mod weather;
pub(crate) mod hera;

#[derive(Debug)]
struct Function<'a> {
    regex: &'a Regex,
    function: fn(&Captures) -> Result<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Property {
    username: String,
    property: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_timestamp: Option<DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

impl Property {
    pub fn new(username: &str, property: &str, value: &str) -> Self {
        Self {
            username: username.to_string(),
            property: property.to_string(),
            value: value.to_string(),
            updated_timestamp: Some(DateTime::now()),
            updated_at: None,
        }
    }

    pub fn json(&mut self) -> String {
        Self::value(&self.property, &self.value)
        // if let Some(updated_timestamp) = self.updated_timestamp {
        //     self.updated_timestamp = None;
        //     self.updated_at = Some(Self::bson_datetime_local(updated_timestamp));
        // }
        // serde_json::to_string(self).unwrap()
    }

    fn _bson_datetime_local(bson_dt: DateTime) -> String {
        let utc_dt = Utc
            .timestamp_opt(bson_dt.timestamp_millis() / 1000, 0)
            .single()
            .unwrap_or(Utc.timestamp_opt(0, 0).unwrap());
        let local_dt = utc_dt.with_timezone(&Local);
        local_dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    pub fn value(property: &str, value: &str) -> String {
        let mut map = HashMap::with_capacity(1);
        map.insert(property, value);
        serde_json::to_string(&map).unwrap()
    }
}
