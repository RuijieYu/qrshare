use std::{net::IpAddr, path::PathBuf};

/// A [`Cli`] is the collection of all options configurable from the
/// command-line arguments.
#[derive(clap::Parser, Debug, Clone)]
#[clap(name = "QR Share")]
#[clap(version = "0.1.0")]
#[clap(author = "Ruijie Yu <ruijie@netyu.xyz>")]
#[clap(about = "qrshare")]
pub struct Cli {
    /// Debug use, print self after parsing.
    #[cfg(debug_assertions)]
    #[clap(long, value_parser)]
    debug: bool,

    /// Image options.  Use PNG format or SVG format to produce the QR code, or
    /// skip producing the QR code at all.
    #[clap(short = 'I', long, arg_enum, default_value = "png")]
    pub image: ImageOptions,

    /// Quiet operation.  Do not warn about missing files.
    #[clap(short, long, value_parser)]
    pub quiet: Option<bool>,

    /// Strict mode.  When enabled, the server exits on any failure in path
    /// resolution and IO.
    #[clap(short, long, value_parser)]
    pub strict: Option<bool>,

    /// Sets a custom port.  Default to 0, where an arbitrary available port is
    /// used.
    #[clap(short, long, value_parser)]
    pub port: Option<u16>,

    /// Sets a custom bound address, default is all available addresses.
    /// UNIMPLEMENTED
    #[clap(short, long, value_parser)]
    pub bind: Option<IpAddr>,

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
        if this.debug {
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

#[derive(clap::ArgEnum, Debug, Clone)]
pub enum ImageOptions {
    Png,
    Svg,
    None,
}

#[cfg(test)]
mod tests {
    use clap::IntoApp;

    #[test]
    fn test_cli() {
        super::Cli::command().debug_assert()
    }
}
