use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fmt::Debug;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, StatusCode};
use sha2::{Digest, Sha512};
use tempfile::tempdir;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::spawn;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::cli::Cli;
use crate::errors::{self, Error};
use crate::file::asy;
use crate::qr::gen::{gen_qr, QrFileType};
use crate::qr::show::qr_show;
use crate::utils::{query_split_opt, status};

/// The default port to listen
const DEFAULT_PORT: u16 = 0;

/// The default buffer size, in bytes
const DEFAULT_BUFSIZE: usize = 1024;

/// A [`Server`] is the server object.
#[derive(Debug)]
pub struct Server {
    /// The parsed port
    pub port: u16,
    /// The bound address
    pub bind: Option<IpAddr>,
    /// The set of paths of files to serve.  This assumes that directory
    /// structure does not change.
    pub files: HashSet<PathBuf>,
    /// The hash digest of all files.  This assumes that files on record do not
    /// change.
    pub digest: HashMap<String, PathBuf>,
    /// The QR code format.
    pub qr: Option<QrFileType>,
}

impl Server {
    /// Validate and convert the command-line options into a full App structure.
    /// In particular, the collection of files is canonicalized, deduplicated,
    /// and ensured to reference valid files.
    pub async fn new(cli: Cli) -> errors::Result<Self> {
        let port = cli.port.unwrap_or(DEFAULT_PORT);
        let bind = cli.bind;
        let digest = Default::default();

        // QR code filetype validation and processing
        let qr = match (cli.no_qrcode, cli.png, cli.svg) {
            // QR disabled
            (Some(true), _, _) => Ok(None),
            // conflict
            (_, Some(true), Some(true)) => Err(errors::Error::ArgConflict),
            // SVG
            (_, _, Some(true)) => Ok(Some(QrFileType::Svg)),
            // PNG
            _ => Ok(Some(QrFileType::Png)),
        }?;

        // Canonicalize paths, and deduplicate the collection -- raise a warning
        // and continue when not in strict mode, and exit when in strict mode.
        let files = {
            let mut files = HashSet::with_capacity(cli.files.len());
            for p in cli.files {
                let path = asy::canonicalize(&p).await;
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
            Ok(Self { port, bind, files, digest, qr })
        }
    }

    /// Prepare the hash digest for each file.
    async fn prepare_digest(mut self) -> Self {
        for path in &self.files {
            if let Ok(mut file) = asy::File::open(path).await {
                if asy::is_multiread_file(&file).await {
                    let mut digest = Sha512::new();
                    let digest: Vec<u8> = loop {
                        // hold the entirety of file data
                        let mut buf = [Default::default(); DEFAULT_BUFSIZE];
                        // update digest for the newly read data
                        match file.read(&mut buf).await {
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

    /// Internal request handler.  The argument `query` is an iterable struct of
    /// pairs (such as vec of `&str` pair, or a map from `&str` to `&str`).
    async fn handle_request<'q>(
        server: &Self,
        query: impl IntoIterator<Item = (&'q str, &'q str)>,
    ) -> errors::Result<Response<Body>> {
        let query: HashMap<_, _> = query.into_iter().collect();
        let path = query
            .get("h")
            .map(|hash| server.digest.get(hash.to_owned()));

        Ok(match path {
            Some(Some(path)) => {
                if let Ok(file) = File::open(path).await {
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let body = Body::wrap_stream(stream);
                    Response::builder()
                        .header(
                            "Content-Disposition",
                            format!(
                                "attachment; filename=\"{}\"",
                                path.iter().last().unwrap().to_string_lossy()
                            ),
                        )
                        .body(body)?

                    // Response::new(body)
                } else {
                    status(StatusCode::NOT_FOUND)
                }
            }
            // no hash from digest
            Some(None) => status(StatusCode::NOT_FOUND),
            // no h=xxx from query
            None => status(StatusCode::UNPROCESSABLE_ENTITY),
        })
    }

    /// Request handler.
    async fn handle(
        server: impl AsRef<Self>,
        _addr: SocketAddr,
        req: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        let server = server.as_ref();

        let resp = match (req.method(), req.uri().path(), req.uri().query()) {
            // ref:
            // https://github.com/hyperium/hyper/blob/master/examples/send_file.rs
            (&Method::GET, "/sha512" | "/sha512/", query) => {
                Self::handle_request(server, query_split_opt(query)).await
            }
            // usually a browser will ask for /favicon.ico -- this is usually
            // not available
            (&Method::GET, "/favicon.ico", _) => {
                Ok(status(StatusCode::NOT_FOUND))
            }
            _ => Ok(status(StatusCode::METHOD_NOT_ALLOWED)),
        };

        Ok(resp.expect("Unexpected HTTP error"))
    }

    /// Start serving the specified files.
    pub(super) async fn start(self) -> errors::Result<()> {
        // XXX: currently the program binds to 0.0.0.0:port
        let addr = SocketAddr::from(([0; 4], self.port));
        let server = spawn(self.prepare_digest());
        let server = server.await?;

        // make server data sharable across threads
        let server = Arc::new(server);

        let (server, service) = (
            server.clone(),
            make_service_fn(move |conn: &AddrStream| {
                let server = server.clone();
                let addr = conn.remote_addr();
                let service = service_fn(move |req| {
                    Self::handle(server.clone(), addr, req)
                });
                async move { Ok::<_, Infallible>(service) }
            }),
        );

        let s = hyper::Server::try_bind(&addr)?.serve(service);

        // start server as a new task in the background
        let listen = s.local_addr();
        println!("Listening on {}", listen);
        let s = spawn(s);

        // Show the QR code
        if let Some(qr) = server.qr {
            // Now that the server has started, generate the QR code for the
            // first path.  When multiple files are supplied, it is unspecified
            // which file is supplied.  We *know* that `let file = _;` must
            // succeed because we require at least one file.
            let file = server.files.iter().next().unwrap();
            let digest = server
                .digest
                .iter()
                // XXX: waiting for bool::then_some(val) to come to stable
                .find_map(|(key, val)| (val == file).then(move || key))
                .expect("The digest for this file should have been generated");

            let dir = tempdir()?;
            let qr_path =
                gen_qr(listen, digest, "sha512", "http", qr, &dir).await?;
            qr_show(qr_path).await?;
        }

        s.await??;

        Ok(())
    }
}
