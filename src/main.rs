mod cli;
mod server;

use crate::{cli::Cli, server::Server};
use lib::errors;

#[tokio::main]
async fn main() -> errors::Result<()> {
    let app = Server::new(Cli::parse()).await?;
    app.start().await?;

    Ok(())
}
