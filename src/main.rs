mod agent;
mod error;
mod logger;
mod slm;

use crate::{
    agent::{Agent, dictionary::DictionaryAgent, prompt::PromptAgent, rag::RagAgent},
    error::Result,
    slm::SlmRequest,
};
use clap::Parser;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use log::trace;
use regex::Regex;
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

    #[arg(long, default_value = "false", help = "Force SLM")]
    force_slm: bool,

    #[arg(short, long, help = "SLM system role")]
    system: Option<String>,

    #[arg(short, long, help = "RAG context name")]
    context: Option<String>,

    // all remaining arguments as prompt
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

lazy_static! {
    pub static ref PATTERNS: Vec<(&'static str, &'static str)> = vec![(
        r"(?i)\b(?:def|define|what\s+is|what\s+.*mean\w*)\s+(?:of\s+)?(.+)\b",
        DictionaryAgent::ID
    )];
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    logger::init(&args.log_level, &args.log_file);
    trace!("main() -> Result<(), Box<dyn std::error::Error>>");

    let mut context: Option<&str> = None;
    let prompt = args.prompt.join(" ");
    let mut capture = String::new();
    let regex_prompt = prompt.clone();

    if args.context.is_none() && !args.force_slm {
        for (pattern, agent_id) in PATTERNS.iter() {
            let regex_instance = Regex::new(&pattern)?;
            if regex_instance.is_match(&prompt) {
                context = Some(agent_id);
                if let Some(captures) = regex_instance.captures(&regex_prompt) {
                    capture = captures[1].to_string();
                    break;
                };
            }
        }
    }
    if context.is_none() {
        context = args.context.as_deref();
    }

    let agent = match context {
        Some(DictionaryAgent::ID) => Agent::Dictionary(DictionaryAgent::new()),
        Some(_) => Agent::Rag(RagAgent::new()),
        None => Agent::Prompt(PromptAgent::new()),
    };
    let context = match context {
        Some(context) => context.to_string(),
        None => String::new(),
    };

    let mut builder = SlmRequest::builder();
    if let Some(system) = args.system {
        builder.system(system);
    }
    builder.prompt(prompt);

    let request = match agent {
        Agent::Prompt(_) => builder.build(),
        Agent::Rag(_) => builder.context(context).build(),
        Agent::Dictionary(_) => builder.system(capture).build(),
    };

    let mut stream = agent.exec(request).await;
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?);
        let _ = io::stdout().flush().await;
    }
    Ok(())
}
