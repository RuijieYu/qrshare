use std::{net::IpAddr, path::PathBuf};

use clap::Parser;

/// A [`Cli`] is the collection of all options configurable from the
/// command-line arguments.
#[derive(Parser, Debug, Clone)]
#[clap(name = "QR Share")]
#[clap(version = "0.1.0")]
#[clap(author = "Ruijie Yu <ruijie@netyu.xyz>")]
#[clap(about = "qrshare")]
pub struct Cli {
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

    /// Use PNG format when generating the QR code.  This is the default.
    /// Conflicts with `--svg`.  Ignored when `--no-qrcode` is set.
    #[clap(long, value_parser)]
    pub png: Option<bool>,

    /// Use SVG format when generating the QR code.  Conflicts with `--png`, and
    /// is ignored when `--no-qrcode` is set.
    #[clap(long, value_parser)]
    pub svg: Option<bool>,

    /// Do not show the QR code.  Overrides `--svg` and `--png`.
    #[clap(short = 'Q', long, value_parser)]
    pub no_qrcode: Option<bool>,

    /// The paths of files to serve.  There should be at least one file to
    /// serve.
    #[clap(value_parser)]
    pub files: Vec<PathBuf>,
}
