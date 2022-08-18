use std::{net::IpAddr, path::PathBuf};

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

    /// Image options.  Use PNG format or SVG format to produce the QR code, or
    /// skip producing the QR code at all.
    #[clap(short = 'I', long, value_enum, default_value = "png")]
    pub image: ImageOptions,

    /// Quiet operation.  Do not warn about missing files.
    #[clap(short, long, value_parser)]
    pub quiet: Option<bool>,

    /// Strict mode.  When enabled, the server exits on any failure in path
    /// resolution and IO.
    #[clap(short, long, value_parser)]
    pub strict: bool,

    /// Bind options, containing the bound host(s) and port.
    #[clap(flatten)]
    pub bind: BindOptions,

    /// The paths of files to serve.  There should be at least one file to
    /// serve.
    #[clap(value_parser)]
    pub files: Vec<PathBuf>,
}

impl Cli {
    #[cfg(debug_assertions)]
    #[inline]
    pub fn parse() -> Self {
        let this = <Self as clap::Parser>::parse();
        if this.debug_print {
            eprintln!("{:#?}", this);
            std::process::exit(0)
        } else {
            this
        }
    }
    #[cfg(not(debug_assertions))]
    #[inline]
    pub fn parse() -> Self {
        <Self as clap::Parser>::parse()
    }
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum ImageOptions {
    Png,
    Svg,
    None,
}

/// Options for interface binding.
#[derive(clap::Args, Debug, Clone)]
pub struct BindOptions {
    /// Sets a custom bound host address, default is all available addresses.
    /// UNIMPLEMENTED
    #[clap(short = 'H', long, value_parser)]
    pub host: Vec<IpAddr>,

    /// Sets a custom port.  Default to 0, where an arbitrary available port is
    /// used.
    #[clap(short, long, value_parser, default_value = "0")]
    pub port: u16,
}

#[cfg(test)]
mod tests {
    use clap::IntoApp;

    #[test]
    fn test_cli() {
        super::Cli::command().debug_assert()
    }
}
