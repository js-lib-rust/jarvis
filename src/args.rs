use std::net::IpAddr;

use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Args {
    #[arg(
        short,
        long,
        default_value = "off",
        help = "Logging level: off, error, warn, info, debug, trace"
    )]
    pub(crate) log_level: String,

    #[arg(
        short = 'f',
        long,
        help = "Logging file path -- if not specified print logs to console"
    )]
    pub(crate) log_file: Option<String>,

    #[arg(
        short,
        long,
        default_value = "192.168.0.5:1965",
        help = "LLM router host address in format hostname:port"
    )]
    pub(crate) router_addr: String,

    #[arg(
        short,
        long,
        default_value = "http://jarvis.local/v1/chat/completions",
        help = "URL for local LLM API."
    )]
    pub(crate) model_url: String,

    #[arg(
        short,
        long,
        default_value = "0.0.0.0",
        help = "Server listening IP address."
    )]
    pub(crate) ip_addr: IpAddr,

    #[arg(short, long, default_value = "3000", help = "Server listening port.")]
    pub(crate) port: u16,
}

impl Args {
    pub(crate) fn parse() -> Self {
        Parser::parse()
    }
}
