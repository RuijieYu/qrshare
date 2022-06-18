use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, StatusCode};
use sha2::{Digest, Sha512};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::spawn;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::cli::Cli;
use crate::errors::{self, Error};
use crate::file::asy;
use crate::utils::query_split_opt;

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
    pub bind: Option<String>,
    /// The set of paths of files to serve.  This assumes that directory
    /// structure does not change.
    pub files: HashSet<PathBuf>,
    /// The hash digest of all files.  This assumes that files on record do not
    /// change.
    pub digest: HashMap<String, PathBuf>,
}

impl Server {
    /// Convert the command-line options into a full App structure.  In
    /// particular, the collection of files is canonicalized, deduplicated, and
    /// ensured to reference valid files.
    pub async fn new(cli: Cli) -> errors::Result<Self> {
        let port = cli.port.unwrap_or(DEFAULT_PORT);
        let bind = cli.bind;
        let digest = Default::default();

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
            Ok(Self { port, bind, files, digest })
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
        resp: &mut Response<Body>,
        query: impl IntoIterator<Item = (&'q str, &'q str)>,
    ) {
        let query: HashMap<_, _> = query.into_iter().collect();
        eprintln!("Path=/sha512, Q={:#?}", query);

        let path = query
            .get("h")
            .map(|hash| server.digest.get(hash.to_owned()));
        eprintln!("H = {:#?}\nO<O<&P>> = {:#?}", server.digest, path);

        match path {
            Some(Some(path)) => {
                eprintln!("Prepare send: {}", path.display());
                let file = tokio::fs::File::open(path).await;
                if let Ok(file) = file {
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let body = Body::wrap_stream(stream);
                    *resp = Response::new(body);
                } else {
                    *resp.status_mut() = StatusCode::NOT_FOUND;
                }
            }
            // no hash from digest
            Some(None) => *resp.status_mut() = StatusCode::NOT_FOUND,
            // no h=xxx from query
            None => *resp.status_mut() = StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    /// Request handler.
    async fn handle(
        server: impl AsRef<Self>,
        _addr: SocketAddr,
        req: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        let server = server.as_ref();

        let mut resp = Response::new(Body::from("Response"));
        match (req.method(), req.uri().path(), req.uri().query()) {
            // ref:
            // https://github.com/hyperium/hyper/blob/master/examples/send_file.rs
            (&Method::GET, "/sha512" | "/sha512/", query) => {
                Self::handle_request(server, &mut resp, query_split_opt(query))
                    .await
            }
            _ => *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED,
        }

        Ok(resp)
    }

    /// Start serving all files.
    pub(super) async fn start(self) -> errors::Result<()> {
        // XXX: currently the program binds to 0.0.0.0:port
        let addr = SocketAddr::from(([0; 4], self.port));
        let server = spawn(self.prepare_digest());
        let server = server.await?;

        // make server data sharable across threads
        let server = Arc::new(server);

        let service = make_service_fn(move |conn: &AddrStream| {
            let server = server.clone();
            let addr = conn.remote_addr();
            let service =
                service_fn(move |req| Self::handle(server.clone(), addr, req));
            async move { Ok::<_, Infallible>(service) }
        });

        let s = hyper::Server::try_bind(&addr)?.serve(service);

        println!("Listening on {}", s.local_addr());

        s.await?;

        Ok(())
    }
}
