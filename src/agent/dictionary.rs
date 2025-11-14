use crate::agent::AgentStream;
use crate::error::AppError;
use crate::slm::SlmRequest;
use async_stream::stream;
use futures::StreamExt;
use log::trace;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};

pub struct DictionaryAgent;

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

impl DictionaryAgent {
    pub const ID: &'static str = "DictionaryAgent";
    const DB_URL: &'static str = "mongodb://localhost:27017";

    pub fn new() -> Self {
        Self {}
    }

    pub async fn exec(&self, request: SlmRequest) -> AgentStream {
        trace!("DictionaryAgent::exec(&self, request: SlmRequest) -> AgentStream");
        Box::pin(stream! {
            let Some(word) = &request.get_system() else {
                yield Err(AppError::Fatal("missing keyword".to_string()));
                return;
            };

            let client = Client::with_uri_str(DictionaryAgent::DB_URL).await?;
            let database = client.database("kb");
            let collection: Collection<Definition> = database.collection("data");

            let filter = doc! {"$text": {"$search":word}};
            let options = FindOptions::builder().build();

            let mut cursor = collection.find(filter, options).await?;
            let mut response = Vec::<String>::new();
            while let Some(result) = cursor.next().await {
                match result {
                    Ok(definition) => {
                        if !definition.meanings.is_empty() {
                            response.push(String::new());
                            response.push(match definition.part_of_speech {
                                Some(part_of_speech) => format!("# {}, {}", definition.word, part_of_speech.to_lowercase()),
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
                            response.push(format!("- __{}__: {}", expression.phrase, expression.definition));
                        }
                        if !definition.expressions.is_empty() {
                            response.push(String::new());
                        }
                    }
                    Err(e) => eprintln!("Error loading document: {}", e),
                }
            }
            yield Ok(response.join("\n"))
        })
    }
}
