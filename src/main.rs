mod cli;
mod server;
mod services;

use crate::{cli::Cli, server::Server};
use lib::errors;
use log::LevelFilter;
use simple_logger::SimpleLogger;

fn main() -> errors::Result<()> {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Debug)
        .with_module_level("qrshare", LevelFilter::Trace)
        .env()
        .init()
        .unwrap();

    main_actix()
}

#[tokio::main]
async fn main_actix() -> errors::Result<()> {
    let server = Server::new(Cli::parse()).await?;
    Server::start_actix(server).await?;

    Ok(())
}
