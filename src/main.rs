mod cli;
mod server;

use clap::Parser;
use qrshare_lib::errors;

use crate::{cli::Cli, server::Server};

#[tokio::main]
async fn main() -> errors::Result<()> {
    let app = Server::new(Cli::parse()).await?;
    app.start().await?;

    Ok(())
}
