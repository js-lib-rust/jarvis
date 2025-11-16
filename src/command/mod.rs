pub mod dictionary;

use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    command::dictionary::DictionaryCommand,
    error::{AppError, Result},
};

pub type CommandBuilder = fn(regex::Captures) -> Command;

pub enum Command {
    DictionaryCommand(DictionaryCommand),
}

impl FromStr for Command {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        for (regex, builder) in COMMAND_REGEX.iter() {
            if let Some(captures) = regex.captures(s) {
                return Ok(builder(captures));
            }
        }
        Err(AppError::ParseCommandError)
    }
}

impl Command {
    pub async fn exec(&self) -> Result<Option<String>> {
        match self {
            Command::DictionaryCommand(command) => command.exec().await,
        }
    }
}

lazy_static! {
    static ref COMMAND_REGEX: Vec<(Regex, CommandBuilder)> =
        vec![(DictionaryCommand::pattern(), DictionaryCommand::parse)];
}
