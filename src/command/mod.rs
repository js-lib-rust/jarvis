pub mod dictionary;
pub mod server;

use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    command::dictionary::DictionaryCommand,
    command::server::ServerCommand,
    error::{AppError, Result},
};

pub type CommandBuilder = fn(regex::Captures) -> Command;

pub enum Command {
    DictionaryCommand(DictionaryCommand),
    ServerCommand(ServerCommand),
}

impl FromStr for Command {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        for (regex, parser) in COMMAND_REGEX.iter() {
            if let Some(captures) = regex.captures(s) {
                return Ok(parser(captures));
            }
        }
        Err(AppError::ParseCommandError)
    }
}

impl Command {
    pub async fn exec(&self) -> Result<Option<String>> {
        match self {
            Command::DictionaryCommand(command) => command.exec().await,
            Command::ServerCommand(command) => command.exec().await,
        }
    }
}

lazy_static! {
    static ref COMMAND_REGEX: Vec<(Regex, CommandBuilder)> = vec![
        (DictionaryCommand::pattern(), DictionaryCommand::parse),
        (ServerCommand::pattern(), ServerCommand::parse),
    ];
}
