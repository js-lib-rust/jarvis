use crate::types::Result;
use crate::{
    error::AppError,
    service::{Function, Property},
    util::string,
};
use bson::DateTime;
use lazy_static::lazy_static;
use log::{debug, trace};
use mongodb::{Client, bson::doc, options::FindOneOptions};
use regex::{Captures, Regex};
use std::vec;

lazy_static! {
    // Set the (social security number) to (1640315227781) of|for (Rotaru Iulian).
    static ref setter_pattern: Regex = Regex::new(r"(?i)^set(?: the)? (.+) to (.+) (?:of|for) (.+).$").unwrap();
    // Get the (social security number) of|for (Rotaru Iulian).
    static ref getter_pattern: Regex = Regex::new(r"(?i)^get(?: the)? (.+) (?:of|for) (.+).$").unwrap();
    // Get my (social security number).
    static ref update_pattern: Regex = Regex::new(r"(?i)^update(?: the)? (.+) to (.+) (?:of|for) (.+).$").unwrap();

    static ref functions: Vec<Function<'static>> = vec![
        Function {regex: &setter_pattern, function: set_property_sync},
        Function {regex: &getter_pattern, function: get_property_sync},
        Function {regex: &update_pattern, function: update_property_sync}
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

const SERVER: &'static str = "mongodb://localhost:27017";
const DATABASE: &'static str = "jarvis";
const COLLECTION: &'static str = "user_profile";

fn get_property_sync(captures: &Captures) -> Result<String> {
    trace!("get_property_sync(captures: &Captures) -> Result<String>");

    let username = &captures[2];
    debug!("username: {}", username);
    let property = &captures[1];
    debug!("property: {}", property);

    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        if let Some(mut property) = handle.block_on(get_property(username, property)) {
            Ok(property.json())
        } else {
            Err(AppError::Fatal(format!(
                "User property {} not found.",
                property
            )))
        }
    })
}

pub async fn get_property(username: &str, property: &str) -> Option<Property> {
    trace!("get_property(username: &str, property: &str) -> Option<Property>");
    debug!("username: {}", username);
    debug!("property: {}", property);

    let client = Client::with_uri_str(SERVER).await.ok()?;
    let database = client.database(DATABASE);
    let collection = database.collection::<Property>(COLLECTION);

    let filter = doc! {"$and":[{"username":username},{"property":&string::snake_case(&property)}]};
    let options = FindOneOptions::builder().build();

    collection.find_one(filter, options).await.ok()?
}

fn set_property_sync(captures: &Captures) -> Result<String> {
    trace!("set_property_sync(captures: &Captures) -> Result<String>");

    let username = &captures[3];
    debug!("username: {}", username);
    let property = &captures[1];
    debug!("property: {}", property);
    let value = &captures[2];
    debug!("value: {}", value);

    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(set_property(username, property, value))
    })
}

pub async fn set_property(username: &str, property: &str, value: &str) -> Result<String> {
    trace!("set_property(username: &str, property: &str, value: &str) -> Result<()>");
    debug!("username: {}", username);
    debug!("property: {}", property);
    debug!("value: {}", value);

    let client = Client::with_uri_str(SERVER).await?;
    let database = client.database(DATABASE);
    let collection = database.collection::<Property>(COLLECTION);

    let property = Property::new(username, &string::snake_case(&property), value);
    let _ = collection.insert_one(property, None).await;

    Ok(String::new())
}

fn update_property_sync(captures: &Captures) -> Result<String> {
    trace!("update_property_sync(captures: &Captures) -> Result<String>");

    let username = &captures[3];
    debug!("username: {}", username);
    let property = &captures[1];
    debug!("property: {}", property);
    let value = &captures[2];
    debug!("value: {}", value);

    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(update_property(username, property, value))
    })
}

async fn update_property(username: &str, property: &str, value: &str) -> Result<String> {
    trace!("update_property(username: &str, property: &str, value: &str) -> Result<String>");
    debug!("username: {}", username);
    debug!("property: {}", property);
    debug!("value: {}", value);

    let client = Client::with_uri_str(SERVER).await?;
    let database = client.database(DATABASE);
    let collection = database.collection::<Property>(COLLECTION);

    let query = doc! {"property":property};
    let update = doc! {"$set":{
        "value": value,
        "updated_timestamp": DateTime::now(),
    }};
    let _ = collection.update_one(query, update, None).await?;

    Ok(String::new())
}
