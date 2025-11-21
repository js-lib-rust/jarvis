use crate::error::Result;
use futures::Stream;
use std::pin::Pin;

pub mod prompt;
pub mod rag;

pub type AgentStream = Pin<Box<dyn Stream<Item = Result<String>>>>;
