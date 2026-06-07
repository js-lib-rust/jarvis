use crate::service::Property;
use crate::types::Result;
use crate::util::string::snake_case;
use bson::DateTime;
use futures::{StreamExt, future::ready, stream::TryStreamExt};
use log::{debug, trace};
use mongodb::{
    Client,
    bson::doc,
    options::{DeleteOptions, FindOneOptions, FindOptions},
};
use serde::Deserialize;
use std::{fmt, vec};

const SERVER: &'static str = "mongodb://localhost:27017";
const DATABASE: &'static str = "jarvis";
const COLLECTION: &'static str = "user_profile";

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum MixedValue {
    String(String),
    Number(f64),
}

impl fmt::Display for MixedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MixedValue::String(s) => write!(f, "{}", s),
            MixedValue::Number(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Deserialize)]
pub struct SetProperty {
    username: String,
    property: String,
    // function LLM can return numeric values even if training set uses only strings
    value: MixedValue,
}

impl SetProperty {
    pub async fn exec(&self) -> Result<String> {
        trace!("SetProperty::exec(&self) -> Result<String>");
        debug!("username: {}", self.username);
        debug!("property: {}", self.property);
        debug!("value: {:?}", self.value);

        let client = Client::with_uri_str(SERVER).await?;
        let database = client.database(DATABASE);
        let collection = database.collection::<Property>(COLLECTION);

        let property = Property::new(
            &self.username,
            &snake_case(&self.property),
            &self.value.to_string(),
        );
        let _ = collection.insert_one(property, None).await;

        Ok(String::new())
    }
}

#[derive(Deserialize)]
pub struct UpdateProperty {
    username: String,
    property: String,
    value: String,
}

impl UpdateProperty {
    pub async fn exec(&self) -> Result<String> {
        trace!("UpdateProperty::exec(&self) -> Result<String>");
        debug!("username: {}", self.username);
        debug!("property: {}", self.property);
        debug!("value: {}", self.value);

        let client = Client::with_uri_str(SERVER).await?;
        let database = client.database(DATABASE);
        let collection = database.collection::<Property>(COLLECTION);

        let query = doc! {"property": &snake_case(&self.property)};
        let update = doc! {"$set": {
            "value": &self.value,
            "updated_timestamp": DateTime::now(),
        }};

        let result = collection.update_one(query, update, None).await?;
        if result.matched_count == 0 {
            return Ok(format!(
                "User {} property {} not found.",
                self.username, self.property
            ));
        }
        if result.modified_count == 0 {
            return Ok(format!(
                "User {} property {} not changed.",
                self.username, self.property
            ));
        }
        Ok(format!(
            "User {} property {} upated to {}.",
            self.username, self.property, self.value
        ))
    }
}

#[derive(Deserialize)]
pub struct RenameProperty {
    username: String,
    old_property: String,
    new_property: String,
}

impl RenameProperty {
    pub async fn exec(&self) -> Result<String> {
        trace!("RenameProperty::exec(&self) -> Result<String>");
        debug!("username: {}", self.username);
        debug!("old_property: {}", self.old_property);
        debug!("new_property: {}", self.new_property);

        let client = Client::with_uri_str(SERVER).await?;
        let database = client.database(DATABASE);
        let collection = database.collection::<Property>(COLLECTION);

        let query = doc! {"property": &snake_case(&self.old_property)};
        let update = doc! {"$set": {
            "property": &snake_case(&self.new_property),
            "updated_timestamp": DateTime::now(),
        }};

        let result = collection.update_one(query, update, None).await?;
        if result.matched_count == 0 {
            return Ok(format!(
                "User {} property {} not found.",
                self.username, self.old_property
            ));
        }
        if result.modified_count == 0 {
            return Ok(format!(
                "User {} property {} name not changed.",
                self.username, self.old_property
            ));
        }
        Ok(format!(
            "User {} property {} renamed to {}.",
            self.username, self.old_property, self.new_property
        ))
    }
}

#[derive(Deserialize)]
pub struct RemoveProperty {
    username: String,
    property: String,
}

impl RemoveProperty {
    pub async fn exec(&self) -> Result<String> {
        trace!("RemoveProperty::exec(&self) -> Result<String>");
        debug!("username: {}", self.username);
        debug!("property: {}", self.property);

        let client = Client::with_uri_str(SERVER).await?;
        let database = client.database(DATABASE);
        let collection = database.collection::<Property>(COLLECTION);

        // AND operator is implicit when filter contains multiple fields
        let filter = doc! {"username": &self.username, "property": &snake_case(&self.property)};
        let options = DeleteOptions::builder().build();

        let result = collection.delete_one(filter, options).await?;
        Ok(if result.deleted_count == 1 {
            format!("User {} property {} deleted.", self.username, self.property)
        } else {
            format!(
                "User {} property {} not found.",
                self.username, self.property
            )
        })
    }
}

#[derive(Deserialize)]
pub struct GetProperty {
    username: String,
    property: String,
}

impl GetProperty {
    pub async fn exec(&self) -> Result<String> {
        trace!("GetProperty::exec(&self) -> Result<String>");
        debug!("username: {}", self.username);
        debug!("property: {}", self.property);

        let client = Client::with_uri_str(SERVER).await?;
        let database = client.database(DATABASE);
        let collection = database.collection::<Property>(COLLECTION);

        let filter = doc! {"$and": [
            {"username": &self.username},
            {"property": &snake_case(&self.property)}
        ]};
        let options = FindOneOptions::builder().build();

        Ok(
            if let Some(mut property) = collection.find_one(filter, options).await? {
                property.json()
            } else {
                format!("User property {} not found.", self.property)
            },
        )
    }
}

#[derive(Deserialize)]
pub struct ListProperties {
    username: String,
}

impl ListProperties {
    pub async fn exec(&self) -> Result<String> {
        trace!("ListProperties::exec(&self) -> Result<String>");
        debug!("username: {}", self.username);

        let client = Client::with_uri_str(SERVER).await?;
        let database = client.database(DATABASE);
        let collection = database.collection::<Property>(COLLECTION);

        let filter = doc! {"username": &self.username};
        let options = FindOptions::builder().build();

        let properties = collection
            .find(filter, options)
            .await?
            .filter_map(|result| ready(result.ok()))
            .map(|property| serde_json::to_string(&property))
            .try_collect::<Vec<String>>()
            .await?
            .join("\n");

        Ok(if properties.is_empty() {
            format!("User {} properties not found.", self.username)
        } else {
            properties
        })
    }
}
