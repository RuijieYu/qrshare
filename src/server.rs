use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    sync::Arc,
};

use actix_service::ServiceFactory;
use actix_web::{dev::ServiceRequest, web::Data, App, HttpServer};
use either::Either;
use futures::stream::FuturesUnordered;
use sha2::{Digest, Sha512};
use tokio::{io::AsyncReadExt, sync::RwLock, task::spawn};

use crate::{
    cli::Cli,
    services::{favicon, list_files_noext, show_qr},
};
use lib::{
    config::{BindOptions, ImageOptions},
    errors::{self, Error},
    file::asy,
};

use super::services::{get_sha512, list_files};

/// The default buffer size, in bytes
const DEFAULT_BUFSIZE: usize = 1024;

/// A [`Server`] is the server object.
#[derive(Debug, Clone)]
pub struct Server {
    /// The bind options
    pub bind: BindOptions,

    /// The QR code format.
    pub qr: ImageOptions,

    /// The collection of file paths queued for serving.  This assumes that the
    /// underlying files are unmodified.
    pub files: Arc<RwLock<VecDeque<PathBuf>>>,

    /// The hash digest of all currently-hashed files.
    pub digest: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl Server {
    /// Validate and convert the command-line options into a full App structure.
    /// In particular, the collection of files is canonicalized, deduplicated,
    /// and ensured to reference valid files.
    pub async fn new(cli: Cli) -> errors::Result<Self> {
        let qr = cli.config.image();
        let bind = cli.config.bind;

        // Canonicalize paths, and deduplicate the collection -- raise a warning
        // and continue when not in strict mode, and exit when in strict mode.
        let files = {
            let mut files = HashSet::with_capacity(cli.files.len());
            for p in cli.files {
                let path = asy::canonicalize(&p).await;
                match (cli.config.strict, cli.config.quiet, path) {
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
            let files = Arc::new(RwLock::new(files.into_iter().collect()));
            Ok(Self { bind, files, digest: Arc::default(), qr })
        }
    }

    /// Queue additional files for serving.  This method will acquire a write
    /// lock on `files`.  Files that cannot be canonicalized are skipped.
    pub async fn enqueue(&self, files: impl IntoIterator<Item = PathBuf>) {
        let mut lock = self.files.write().await;
        for path in files.into_iter() {
            if let Ok(canon_path) = asy::canonicalize(&path).await {
                log::info!(
                    "Enqueuing path: {} ({})",
                    path.display(),
                    canon_path.display()
                );
                lock.push_back(canon_path)
            } else {
                log::error!(
                    "Failed to canonicalize path, skipping: {}",
                    path.display()
                );
            }
        }
    }

    /// Process all queued files.  This method will acquire a write lock on
    /// `files`, and also a write lock on `digest`.  When this function returns,
    /// the queue will become emtpy.  If `skip_existing` is true, then skip a
    /// path when the entry already exists in the digest database.
    ///
    /// TODO: actually implement `skip_existing`
    pub async fn process_digest(self: Arc<Self>) -> errors::Result<()> {
        let futs = FuturesUnordered::new();
        while let Some(path) = self.files.write().await.pop_front() {
            let this = self.clone();
            futs.push(spawn(async move {
                log::trace!("Beginning processing {}", path.display());

                if let Ok(mut file) = asy::File::open(&path).await {
                    if asy::is_multiread_file(&file).await {
                        let mut d = Sha512::new();
                        let d: Vec<_> = loop {
                            // hold the entirety of file data
                            let mut buf = [0; DEFAULT_BUFSIZE];
                            // update digest for the newly read data
                            match file.read(&mut buf).await {
                                // EOF or error
                                Ok(0) | Err(_) => break d.finalize(),
                                Ok(sz) => d.update(&buf[0..sz]),
                            }
                        }
                        .into_iter()
                        .collect();

                        // get the digest string, and store into hash table when
                        // empty
                        this.digest
                            .write()
                            .await
                            .insert(hex::encode(d), path.clone());
                    }
                }

                log::trace!("Finished processing {}", path.display())
            }))
        }

        for fut in futs {
            fut.await?
        }

        Ok(())
    }

    /// Query for an existing digest from the path.
    pub async fn query_digest(&self, path: PathBuf) -> Option<String> {
        self.digest
            .read()
            .await
            .iter()
            .find_map(|(d, p)| (*p == path).then(|| d.clone()))
    }

    /// Construct the URL for a given file path (left) or digest (right)
    pub async fn file_url(
        &self,
        file: Either<String, PathBuf>,
    ) -> Option<String> {
        Some(format!(
            "{}://{}:{}/{}/?h={}",
            "http",
            self.bind.primary_host(),
            self.bind.port(),
            "sha512",
            match file {
                Either::Left(digest) => digest,
                Either::Right(path) => self.query_digest(path).await?,
            }
        ))
    }

    /// Construct the QR code URL for a given file path (left) or digest
    /// (right).  The URL format is "/qr/{method}/h?{hash}".
    pub async fn qr_url(
        &self,
        file: Either<String, PathBuf>,
    ) -> Option<String> {
        Some(format!(
            "{}://{}:{}/qr/{}/?h={}",
            "http",
            self.bind.primary_host(),
            self.bind.port(),
            "sha512",
            match file {
                Either::Left(digest) => digest,
                Either::Right(path) => self.query_digest(path).await?,
            }
        ))
    }

    /// Server builder function for [`actix_web`].
    fn http_builder<T>(server: Data<Self>, app: App<T>) -> App<T>
    where
        T: ServiceFactory<
            ServiceRequest,
            Config = (),
            Error = actix_web::Error,
            InitError = (),
        >,
    {
        app.app_data(server)
            // main services
            .service(get_sha512)
            .service(list_files)
            .service(favicon)
            .service(show_qr)
            // redirect (alias) services
            .service(list_files_noext)
    }

    /// The entry point to start the file server with [`actix_web`].
    pub async fn start_actix(self) -> errors::Result<()> {
        // listen the specified TCP ports
        let port = self.bind.port();
        let listen = self.bind.hosts_iter().flat_map(|ip| {
            TcpListener::bind(SocketAddr::from((ip, port))).ok()
        });

        // wrap to web data
        let this = Data::new(self);

        // process queued files
        Arc::clone(&this).process_digest().await?;

        // create the HTTP server
        let http_server = {
            let mut http_server = HttpServer::new(move || {
                Self::http_builder(Data::clone(&this), App::new())
            });
            for listen in listen {
                http_server = http_server.listen(listen)?
            }
            http_server
        };

        log::trace!("Starting HTTP server");
        http_server.run().await?;

        Ok(())
    }
}
