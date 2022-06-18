#![allow(dead_code)]

mod cli;
pub mod errors;
mod file;
mod route;
mod server;
mod utils;

use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> errors::Result<()> {
    let app = crate::server::Server::new(Cli::parse()).await?;
    app.start().await?;
    Ok(())
}
