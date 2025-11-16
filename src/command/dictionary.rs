use futures::StreamExt;
use log::trace;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::{Client, Collection};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::error::Result;

pub struct DictionaryCommand {
    word: String,
}

impl DictionaryCommand {
    const CONNECTION_URL: &'static str = "mongodb://localhost:27017";
    const DATABASE: &'static str = "kb";
    const COLLECTION: &'static str = "data";

    pub fn pattern() -> Regex {
        Regex::new(r"^(?i)\b(?:def|define)\s+(?:of\s+)?(.+)\b$").unwrap()
    }

    pub fn parse(captures: Captures) -> Command {
        trace!("DictionaryCommand::parse(captures: Captures) -> Command");
        let word = captures[1].to_string();
        let command = DictionaryCommand { word };
        Command::DictionaryCommand(command)
    }

    pub async fn exec(&self) -> Result<Option<String>> {
        trace!("DictionaryCommand::exec(&self) -> exec(&self) -> Result<Option<String>>");
        let client = Client::with_uri_str(Self::CONNECTION_URL).await?;
        let database = client.database(Self::DATABASE);
        let collection: Collection<Definition> = database.collection(Self::COLLECTION);

        let filter = doc! {"$text": {"$search": &self.word}};
        let options = FindOptions::builder().build();

        let mut cursor = collection.find(filter, options).await?;
        let mut response = Vec::<String>::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(definition) => {
                    if !definition.meanings.is_empty() {
                        response.push(String::new());
                        response.push(match definition.part_of_speech {
                            Some(part_of_speech) => {
                                format!("# {}, {}", definition.word, part_of_speech.to_lowercase())
                            }
                            None => format!("# {}", definition.word),
                        });
                        response.push(String::new());

                        let mut numbering = 1;
                        for meaning in definition.meanings {
                            response.push(format!("{}. {}", numbering, meaning.definition));
                            response.push(String::new());

                            for example in meaning.examples {
                                response.push(format!("> {}  ", example.text));
                                if let Some(source) = example.source {
                                    response.push(format!("\u{2014} _{}_", source));
                                }
                                response.push(String::new());
                            }
                            numbering = numbering + 1;
                        }

                        if !definition.expressions.is_empty() {
                            response.push(String::from("## În expresie"));
                        }
                    }

                    for expression in &definition.expressions {
                        response.push(format!(
                            "- __{}__: {}",
                            expression.phrase, expression.definition
                        ));
                    }
                    if !definition.expressions.is_empty() {
                        response.push(String::new());
                    }
                }
                Err(e) => eprintln!("Error loading document: {}", e),
            }
        }

        if response.is_empty() {
            Ok(None)
        } else {
            Ok(Some(response.join("\n")))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Definition {
    word: String,
    key: String,
    part_of_speech: Option<String>,
    meanings: Vec<Meaning>,
    expressions: Vec<Expression>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Expression {
    phrase: String,
    definition: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Example {
    text: String,
    source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Meaning {
    definition: String,
    examples: Vec<Example>,
}
