use std::path::PathBuf;

use lib::config::Config;
use log::Level;

/// A [`Cli`] is the collection of all options configurable from the
/// command-line arguments.
#[derive(clap::Parser, Debug, Clone)]
#[clap(name = "QR Share")]
#[clap(version = "0.1.0")]
#[clap(author = "Ruijie Yu <ruijie@netyu.xyz>")]
#[clap(about = "qrshare")]
pub struct Cli {
    /// Debug use only: print self after parsing, and terminate.
    #[cfg(debug_assertions)]
    #[clap(long, value_parser)]
    debug_print: bool,

    /// General configurations, which may come from a configuration file.
    #[clap(flatten)]
    pub config: Config,

    /// The paths of files to serve.  There should be at least one file to
    /// serve.
    #[clap(value_parser)]
    pub files: Vec<PathBuf>,

    /// The log level to use.
    #[clap(short = 'L', long, value_parser, default_value_t = Level::Warn)]
    pub log_level: Level,
}

impl Cli {
    #[cfg(debug_assertions)]
    #[inline]
    pub fn parse() -> Self {
        let this = <Self as clap::Parser>::parse();
        if this.debug_print {
            eprintln!("{:#?}", this);
            std::process::exit(0)
        }
        this
    }
    #[cfg(not(debug_assertions))]
    #[inline]
    pub fn parse() -> Self {
        <Self as clap::Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use clap::IntoApp;

    #[test]
    fn test_cli() {
        super::Cli::command().debug_assert()
    }
}
