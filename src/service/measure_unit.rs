use std::{collections::HashMap, vec};

use crate::{error::AppError, service::Function, types::Result, util::string};
use bson::DateTime;
use lazy_static::lazy_static;
use log::{debug, trace};
use mongodb::{Client, bson::doc, options::FindOneOptions};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

lazy_static! {
    // Get the measurement units for (distance).
    static ref getter_pattern: Regex = Regex::new(r"(?i)^get the measure units for (.+).$").unwrap();

    static ref functions: Vec<Function<'static>> = vec![
        Function {regex: &getter_pattern, function: get_units_sync},
    ];
}

pub fn exec(prompt: &str) -> Result<String> {
    trace!("exec(prompt: &str) -> Result<String>");

    for f in functions.iter() {
        debug!("f: {:?}", f);
        if let Some(captures) = f.regex.captures(prompt) {
            debug!("captures: {:?}", captures);
            debug!("captures length: {}", captures.len());
            return (f.function)(&captures);
        }
    }

    Ok(String::new())
}

#[derive(Serialize, Deserialize, Debug)]
struct Record {
    physical_quantity: String,
    measure_unit: String,
    unit_description: String,
    updated_timestamp: DateTime,
}

impl Record {
    pub fn json(&mut self) -> String {
        Self::value(&self.physical_quantity, &self.measure_unit)
    }

    pub fn value(property: &str, value: &str) -> String {
        let mut map = HashMap::with_capacity(1);
        let property = format!("{}_units", property);
        map.insert(&property, value);
        serde_json::to_string(&map).unwrap()
    }
}

const SERVER: &'static str = "mongodb://localhost:27017";
const DATABASE: &'static str = "jarvis";
const COLLECTION: &'static str = "measure_unit";

fn get_units_sync(captures: &Captures) -> Result<String> {
    trace!("get_units_sync(captures: &Captures) -> Result<String>");

    let physical_quantity = &captures[1];
    debug!("physical_quantity: {}", physical_quantity);

    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        if let Some(mut record) = handle.block_on(get_units(physical_quantity)) {
            Ok(record.json())
        } else {
            Err(AppError::Fatal(format!(
                "Measure unit for {} not found.",
                physical_quantity
            )))
        }
    })
}

async fn get_units(physical_quantity: &str) -> Option<Record> {
    trace!("get_units(physical_quantity: &str) -> Option<Record>");
    debug!("physical_quantity: {}", physical_quantity);

    let client = Client::with_uri_str(SERVER).await.ok()?;
    let database = client.database(DATABASE);
    let collection = database.collection::<Record>(COLLECTION);

    let filter = doc! {"physical_quantity":&string::snake_case(physical_quantity)};
    let options = FindOneOptions::builder().build();

    collection.find_one(filter, options).await.ok()?
}
