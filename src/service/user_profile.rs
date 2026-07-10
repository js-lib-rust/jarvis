use crate::{service::Property, types::Result};
use crate::util::string::snake_case;
use bson::DateTime;
use futures::{StreamExt, future::ready, stream::TryStreamExt};
use log::{debug, trace};
use mongodb::Collection;
use mongodb::{Client, bson::doc};
use serde::Deserialize;
use std::fmt;
use tokio::sync::OnceCell;

const SERVER: &str = "mongodb://localhost:27017";
const DATABASE: &str = "jarvis";
const COLLECTION: &str = "user_profile";

static MONGO_CLIENT: OnceCell<Client> = OnceCell::const_new();
static MONGO_COLLECTION: OnceCell<Collection<Property>> = OnceCell::const_new();

async fn collection() -> Result<&'static Collection<Property>> {
    let client = MONGO_CLIENT
        .get_or_try_init(|| async { Client::with_uri_str(SERVER).await })
        .await?;

    let collection = MONGO_COLLECTION
        .get_or_init(|| async { client.database(DATABASE).collection::<Property>(COLLECTION) })
        .await;

    Ok(collection)
}

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

        let property = Property::new(
            &self.username,
            &snake_case(&self.property),
            &self.value.to_string(),
        );

        collection().await?.insert_one(&property, None).await?;
        Ok(serde_json::to_string(&property)?)
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

        let query = doc! {"username": &self.username, "property": &snake_case(&self.property)};
        let update = doc! {"$set": {
            "value": &self.value,
            "updated_timestamp": DateTime::now(),
        }};

        let result = collection().await?.update_one(query, update, None).await?;
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
            "User {} property {} updated to {}.",
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

        let query = doc! {"username": &self.username, "property": &snake_case(&self.old_property)};
        let update = doc! {"$set": {
            "property": &snake_case(&self.new_property),
            "updated_timestamp": DateTime::now(),
        }};

        let result = collection().await?.update_one(query, update, None).await?;
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

        // AND operator is implicit when filter contains multiple fields
        let filter = doc! {"username": &self.username, "property": &snake_case(&self.property)};
        let result = collection().await?.delete_one(filter, None).await?;

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

        let filter = doc! {"username": &self.username, "property": &snake_case(&self.property)};
        Ok(
            if let Some(mut property) = collection().await?.find_one(filter, None).await? {
                property.json()
            } else {
                format!(
                    "User {} property {} not found.",
                    self.username, self.property
                )
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

        let filter = doc! {"username": &self.username};
        let properties = collection()
            .await?
            .find(filter, None)
            .await?
            // this function is designed for best effort so we can silently ignore deserialization errors
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
