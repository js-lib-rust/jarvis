mod agent;
mod api;
mod args;
mod error;
mod proc;
mod llm;
mod llm_router;
mod logger;
mod service;
mod slm;
mod sys;
mod types;
mod util;

use crate::args::Args;
use crate::types::{AppContext, Result};
use log::{debug, trace};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    logger::init(&args.log_level, &args.log_file);
    trace!("main() -> Result<()>");

    let app_context = AppContext::create(&args.router_addr, &args.model_url).await?;
    debug!("app_context: {:?}", app_context);

    let socket_addr = SocketAddr::from((args.ip_addr, args.port));
    debug!("socket_addr: {}", socket_addr);
    let tcp_listener = tokio::net::TcpListener::bind(socket_addr).await?;
    debug!("tcp_listener: {:?}", tcp_listener);
    let rest_controller = api::create_router(app_context);
    debug!("rest_controller: {:?}", rest_controller);
    Ok(axum::serve(tcp_listener, rest_controller).await?)
}
