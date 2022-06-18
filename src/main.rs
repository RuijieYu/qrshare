#![allow(dead_code)]

pub mod errors;

use clap::Parser;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response};
use sha2::Digest;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fmt::Debug;
use std::fs::{canonicalize, File, FileType};
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::spawn;

use crate::errors::Error;

/// The default port to listen
const DEFAULT_PORT: u16 = 0;

/// The default buffer size, in bytes
const DEFAULT_BUFSIZE: usize = 1024;

/// A [`Cli`] is the collection of all options configurable from the
/// command-line arguments.
#[derive(Parser, Debug, Clone)]
#[clap(name = "QR Share")]
#[clap(version = "0.1.0")]
#[clap(author = "Ruijie Yu <ruijie@netyu.xyz>")]
#[clap(about = "qrshare")]
struct Cli {
    /// Quiet operation.  Do not warn about missing files.
    #[clap(short, long, value_parser)]
    quiet: Option<bool>,

    /// Strict mode.  When enabled, the server exits on any failure in path
    /// resolution and IO.
    #[clap(short, long, value_parser)]
    strict: Option<bool>,

    /// Sets a custom port.  Default to 0, where an arbitrary available port is
    /// used.
    #[clap(short, long, value_parser)]
    port: Option<u16>,

    /// Sets a custom bound address, default is all available addresses.
    /// UNIMPLEMENTED
    #[clap(short, long, value_parser)]
    bind: Option<String>,

    /// The paths of files to serve.  There should be at least one file to
    /// serve.
    #[clap(value_parser)]
    files: Vec<PathBuf>,
}

/// A [`Server`] is the server object.
#[derive(Debug)]
struct Server {
    /// The parsed port
    port: u16,
    /// The bound address
    bind: Option<String>,
    /// The set of paths of files to serve.  This assumes that directory
    /// structure does not change.
    files: HashSet<PathBuf>,
    /// The hash digest of all files.  This assumes that files on record do not
    /// change.
    digest: HashMap<String, PathBuf>,
}

// /// A service context required by [`hyper`].
// struct Context<'s>(&'s Server);

impl Server {
    /// Convert the command-line options into a full App structure.  In
    /// particular, the collection of files is canonicalized, deduplicated, and
    /// ensured to reference valid files.
    fn new(cli: Cli) -> errors::Result<Self> {
        let port = cli.port.unwrap_or(DEFAULT_PORT);
        let bind = cli.bind;
        let digest = Default::default();

        // Canonicalize paths, and deduplicate the collection -- raise a warning
        // and continue when not in strict mode, and exit when in strict mode.
        let files = {
            let mut files = HashSet::with_capacity(cli.files.len());
            for p in cli.files {
                let path = canonicalize(&p);
                match (cli.strict, cli.quiet, path) {
                    // when got a canonicalized path, insert
                    (_, _, Ok(path)) => {
                        files.insert(path);
                    }
                    // when strict + no canonical path, return
                    (Some(true), _, Err(_)) => Err(Error::InvalidFile(p))?,
                    // when not strict + no canonical path + quiet, skip
                    (_, Some(true), Err(_)) => (),
                    // when not strict + no canonical path + not quiet, warn
                    (_, _, Err(_)) => eprintln!("{}", Error::InvalidFile(p)),
                }
            }
            files
        };

        // There should be at least one file to serve
        if files.is_empty() {
            Err(Error::NoFiles)
        } else {
            Ok(Self { port, bind, files, digest })
        }
    }

    /// Check whether a file is a multi-read file.
    fn is_multiread_file(file: &File) -> bool {
        file.metadata()
            .map(|md| md.file_type())
            .map_or(false, Self::is_multiread_md)
    }

    /// Check whether a file type does not represent a single-read file.
    #[cfg(target_family = "unix")]
    fn is_multiread_md(ft: FileType) -> bool {
        use std::os::unix::fs::FileTypeExt;
        !ft.is_fifo() && !ft.is_socket()
    }

    /// Check whether a file type does not represent a single-read file.
    #[cfg(target_os = "wasi")]
    fn is_multiread_md(ft: FileType) -> bool {
        use std::os::wasi::fs::FileTypeExt;
        !ft.is_socket()
    }

    /// Check whether a file type does not represent a single-read file.
    #[cfg(windows)]
    fn is_multiread_md(_: FileType) -> bool {
        true
    }

    /// Prepare the hash digest for each file.
    async fn prepare_digest(mut self) -> Self {
        for path in &self.files {
            if let Ok(mut file) = File::open(path) {
                if Self::is_multiread_file(&file) {
                    let mut digest = sha2::Sha512::new();
                    let digest: Vec<u8> = loop {
                        // hold the entirety of file data
                        let mut buf = [Default::default(); DEFAULT_BUFSIZE];
                        // update digest for the newly read data
                        match file.read(&mut buf) {
                            // EOF
                            Ok(0) => break digest.finalize(),
                            // .read() *may* return a larger sz than capacity
                            Ok(sz) => {
                                digest.update(&buf[0..min(sz, buf.len())])
                            }
                            Err(_) => break digest.finalize(),
                        }
                    }
                    .into_iter()
                    .collect();

                    // get the digest, and store into hash table when empty
                    let digest = hex::encode(digest);
                    self.digest.entry(digest).or_insert_with(|| path.clone());
                }
            }
        }
        self
    }

    /// Request handler.
    async fn handle(
        server: impl AsRef<Self>,
        addr: SocketAddr,
        req: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        let server = server.as_ref();
        let headers = req.headers();
        let body = req.body();

        eprintln!(
            concat!(
                "\tH: {:#?}\n\n",
                "\tB: {:#?}\n\n",
                "\tS: {:#?}\n\n",
                "\tA: {:#?}",
            ),
            headers, body, server, addr
        );

        Ok(Response::new("Response".into()))
    }

    /// Start serving all files.
    async fn start(self) -> errors::Result<()> {
        // XXX: currently the program binds to 0.0.0.0:port
        let addr = SocketAddr::from(([0; 4], self.port));
        let server = spawn(self.prepare_digest());
        let server = server.await?;

        // make server data sharable across threads
        let server = Arc::new(server);

        let service = make_service_fn(move |conn: &AddrStream| {
            println!("Listening on {}", conn.local_addr());

            let server = server.clone();
            let addr = conn.remote_addr();
            let service =
                service_fn(move |req| Self::handle(server.clone(), addr, req));
            async move { Ok::<_, Infallible>(service) }
        });

        hyper::Server::try_bind(&addr)?.serve(service).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> errors::Result<()> {
    let app = Server::new(Cli::parse())?;
    app.start().await?;
    Ok(())
}
