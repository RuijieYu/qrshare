mod cli;
pub mod errors;
mod file;
mod net;
mod qr;
mod server;
mod utils;

use clap::Parser;

use crate::cli::Cli;
use crate::server::Server;

#[tokio::main]
async fn main() -> errors::Result<()> {
    net::get_first_net(net::is_global_4);

    let app = Server::new(Cli::parse()).await?;
    app.start().await?;

    Ok(())
}
