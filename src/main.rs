mod agent;
mod command;
mod error;
mod logger;
mod slm;

use crate::{
    agent::rag::RagAgent,
    command::Command,
    error::Result,
    slm::{SlmClient, SlmRequest},
};
use clap::Parser;
use futures::Stream;
use futures_util::StreamExt;
use log::trace;
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

    let processed = match context {
        Some(_) => process(RagAgent::new().exec(&mut request).await).await,
        None => false,
    };

    if !processed {
        let request = SlmRequest::new(&prompt);
        let _ = process(SlmClient::new().exec(&request).await).await;
    }

    Ok(())
}

fn process<S>(stream: S) -> impl Future<Output = bool>
where
    S: Stream<Item = Result<String>> + Unpin,
{
    stream.fold(false, |_, chunk| async move {
        match chunk {
            Ok(chunk) => {
                print!("{}", chunk);
                let _ = io::stdout().flush().await;
                true
            }
            Err(error) => {
                println!("fail to process stream chunk: {error}");
                false
            }
        }
    })
}
