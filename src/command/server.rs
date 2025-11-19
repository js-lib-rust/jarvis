use std::io::{Write, stdout};

use log::{debug, trace};
use regex::{Captures, Regex};
use reqwest::Client;

use crate::command::Command;
use crate::error::Result;
use crate::util::net;

pub struct ServerCommand {
    command: String,
    sub_command: Option<String>,
}

impl ServerCommand {
    pub fn pattern() -> Regex {
        Regex::new(
            r"(?i)^(server|history|start|stop|wake-up|clear|shutdown|sleep|status)(?:\s+(status|start|stop|clear|reset|history))?$",
        )
        .unwrap()
    }

    pub fn parse(captures: Captures) -> Command {
        trace!("ServerCommand::parse(captures: Captures) -> Command");
        let command = captures[1].to_string();
        let sub_command = captures.get(2).map(|m| m.as_str().to_string());
        debug!("command: {command}, sub_command: {sub_command:?}");
        Command::ServerCommand(Self {
            command,
            sub_command,
        })
    }

    pub async fn exec(&self) -> Result<Option<String>> {
        trace!("ServerCommand::exec(&self) -> Result<Option<String>>");
        if let Some(sub_command) = &self.sub_command {
            match (self.command.as_str(), sub_command.as_str()) {
                ("server", "start") => self.server_start().await,
                ("server", "stop") => self.server_stop().await,
                ("server", "status") => self.server_status(),
                ("history", "clear" | "reset") | ("clear", "history") => self.history_reset().await,
                _ => Ok(None),
            }
        } else {
            match self.command.as_str() {
                "shutdown" | "sleep" | "stop" => self.server_stop().await,
                "start" | "wake-up" => self.server_start().await,
                "status" => self.server_status(),
                _ => Ok(None),
            }
        }
    }

    async fn server_start(&self) -> Result<Option<String>> {
        trace!("ServerCommand::server_start(&self) -> Result<Option<String>>");
        net::send_wol("10-7c-61-5f-10-be")?;

        self.print("Waiting for server to start ");
        for _ in 0..100 {
            if net::connect_timeout("192.168.0.5", 22, 1000) {
                break;
            }
            self.print(".");
        }
        println!();

        self.print("Waiting for SLM service to start ");
        for _ in 0..100 {
            if net::connect_timeout("192.168.0.5", 1964, 500) {
                break;
            }
            self.print(".");
        }
        println!();

        Ok(Some("SLM server and service started".to_string()))
    }

    fn print(&self, s: &str) {
        print!("{}", s);
        let _ = stdout().flush();
    }

    async fn server_stop(&self) -> Result<Option<String>> {
        trace!("ServerCommand::server_stop(&self) -> Result<Option<String>>");
        let url = "http://192.168.0.5:1964/shutdown";
        let client = Client::new();
        let _ = client.post(url).send().await;
        Ok(Some(String::new()))
    }

    fn server_status(&self) -> Result<Option<String>> {
        trace!("ServerCommand::server_status(&self) -> Result<Option<String>>");
        fn status(status: bool) -> &'static str {
            match status {
                true => "up",
                false => "down",
            }
        }
        let server_status = status(net::connect_timeout("192.168.0.5", 22, 100));
        let slm_status = status(net::connect_timeout("192.168.0.5", 1964, 100));
        let status = format!("- JARVIS server is {server_status}\n- SLM service is {slm_status}");
        Ok(Some(status))
    }

    async fn history_reset(&self) -> Result<Option<String>> {
        trace!("ServerCommand::history_reset(&self) -> Result<Option<String>>");
        let url = "http://192.168.0.5:1964/history/clear";
        let client = Client::new();
        let response = client.post(url).send().await?;
        debug!("history reset response: {response:?}");
        Ok(Some(String::new()))
    }
}
