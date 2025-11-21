mod agent;
mod command;
mod config;
mod error;
mod logger;
mod slm;
mod util;

use crate::{
    agent::{prompt::PromptAgent, rag::RagAgent},
    command::Command,
    error::Result,
    slm::SlmRequest,
};
use clap::Parser;
use futures_util::StreamExt;
use log::{debug, trace};
use tokio::io::{self, AsyncWriteExt};

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short = 'v',
        long,
        default_value = "off",
        help = "Logging level: off, error, warn, info, debug, trace"
    )]
    log_level: String,

    #[arg(
        short = 'f',
        long,
        help = "Logging file path -- if not specified print logs to console"
    )]
    log_file: Option<String>,

    #[arg(long, default_value = "config.yml", help = "config file path")]
    config_file: String,

    #[arg(long, default_value = "false", help = "force SLM")]
    force_slm: bool,

    #[arg(short, long, help = "SLM system role")]
    system: Option<String>,

    #[arg(short, long, help = "RAG context name")]
    context: Option<String>,

    // all remaining arguments as prompt
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

impl Args {
    fn prompt(&self) -> String {
        let mut prompt = String::new();
        let mut iter = self.prompt.iter();

        if let Some(first) = iter.next() {
            if first.trim() != "," {
                prompt.push_str(first);
            }
        }
        for part in iter {
            if !prompt.is_empty() {
                prompt.push(' ');
            }
            prompt.push_str(part);
        }

        prompt
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    logger::init(&args.log_level, &args.log_file);
    trace!("main() -> Result<()>");

    config::init_config(&args.config_file)?;
    let config = config::get_config();
    debug!("config: {config:?}");

    let prompt = args.prompt();
    if let Some(command) = prompt.parse::<Command>().ok() {
        if let Some(response) = command.exec().await? {
            println!("{response}");
            return Ok(());
        };
    }

    let context = args.context.clone();
    let mut request = SlmRequest::new(&prompt);
    if let Some(system) = args.system {
        request.set_system(&system);
    }
    if let Some(context) = args.context {
        request.set_context(&context);
    }

    let mut stream = match context {
        Some(_) => RagAgent::new().exec(request).await,
        None => PromptAgent::new().exec(request).await,
    };

    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
        let _ = io::stdout().flush().await;
    }

    Ok(())
}
