mod agent;
mod api;
mod args;
mod config;
mod error;
mod llm;
mod logger;
mod proc;
mod service;
mod types;
mod util;

use crate::args::Args;
use crate::config::Config;
use crate::types::{AppState, Result};
use log::{debug, trace};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    logger::init(&args.log_level, &args.log_file);
    trace!("main() -> Result<()>");

    Config::load(&args.config_file)?;
    debug!("config: {:?}", Config::get());

    let app_state = AppState::create(&args.router_addr, &args.tool_url, &args.model_url).await?;
    debug!("app_state: {:?}", app_state);

    let socket_addr = SocketAddr::from((args.ip_addr, args.port));
    debug!("socket_addr: {}", socket_addr);
    let tcp_listener = tokio::net::TcpListener::bind(socket_addr).await?;
    debug!("tcp_listener: {:?}", tcp_listener);
    let rest_controller = api::create_router(app_state.clone());
    debug!("rest_controller: {:?}", rest_controller);

    let result = axum::serve(tcp_listener, rest_controller).await?;
    app_state.dispose().await;
    Ok(result)
}
