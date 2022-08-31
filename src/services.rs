//! This module defines HTTP services for actix-web.  See [`actix_web::Handler`]
//! for further information.

use std::path::PathBuf;

use actix_http::StatusCode;
use actix_web::{
    get, post,
    web::{Data, Json, Query},
    HttpResponse, Responder,
};
use either::Either;

use crate::Server;
use lib::errors;

#[derive(serde::Deserialize)]
struct GetQuery {
    #[serde(rename = "h")]
    digest: String,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Enqueue {
    Single { path: PathBuf },
    Multiple { path: Vec<PathBuf> },
}

type Iter<II> = <II as IntoIterator>::IntoIter;
type EnqueueIter = Either<Iter<[PathBuf; 1]>, Iter<Vec<PathBuf>>>;

impl Enqueue {
    pub fn into_paths(self) -> EnqueueIter {
        match self {
            Self::Single { path } => Either::Left([path].into_iter()),
            Self::Multiple { path } => Either::Right(path.into_iter()),
        }
    }
}
impl IntoIterator for Enqueue {
    type Item = PathBuf;
    type IntoIter = EnqueueIter;
    fn into_iter(self) -> Self::IntoIter {
        self.into_paths()
    }
}

#[get("/sha512/")]
#[inline]
async fn get_sha512(
    query: Query<GetQuery>,
    server: Data<Server>,
) -> impl Responder {
    log::trace!("get_sha512()");
    inner::do_get_sha512(query, server).await
}

/// Default service: list all available files.  See also [`list_files`].
pub async fn default_service() -> impl Responder {
    log::trace!("list_files_noext()");
    HttpResponse::PermanentRedirect()
        .append_header(("Location", "/list.html"))
        .finish()
}

#[get("/list.html")]
#[inline]
async fn list_files(server: Data<Server>) -> errors::Result<impl Responder> {
    log::trace!("list_files()");
    inner::do_list_files(server).await
}

/// Whether we should forbid remote file enqueuing.  Forbidding remote file
/// enqueuing *should* still allow "local" (127.0.0.1, ::1) connections to
/// enqueue the files?  Or maybe just add HTTP authentication and call it good.
///
/// This probably needs more thoughts.  TODO
const FORBID_REMOTE_ENQUEUE: bool = !cfg!(feature = "insecure");

/// # SECURITY NOTE
///
/// Care must be taken here.  By allowing this API, we are essentially allowing
/// a remote user to retrieve all files accessible to the current user.
///
/// For now, this is only allowed with feature "insecure".
#[post("/serve")]
#[inline]
async fn enqueue_file(
    server: Data<Server>,
    body: Json<Enqueue>,
) -> impl Responder {
    log::trace!("enqueue_file()");

    if FORBID_REMOTE_ENQUEUE {
        log::trace!("enqueue_file() is forbidden.");
        Err(StatusCode::FORBIDDEN.into())
    } else {
        inner::do_enqueue_file(server, body).await
    }
}

/// Favicon
#[get("/favicon.ico")]
#[inline]
async fn favicon(_: Data<Server>) -> impl Responder {
    log::trace!("favicon()");
    inner::serve_file_at("favicon.ico".as_ref()).await
}

/// Show QR code image
#[get("/qr/sha512/")]
#[inline]
async fn show_qr(
    server: Data<Server>,
    query: Query<GetQuery>,
) -> impl Responder {
    log::trace!("show_qr()");
    inner::do_show_qr(server, query).await
}

mod inner {
    //! Implementation for services.

    use std::{
        ffi::OsStr,
        fmt::Display,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use actix_files::NamedFile;
    use actix_http::StatusCode;
    use actix_web::{
        http::header::ContentType,
        web::{Data, Json, Query},
        HttpResponse, Responder,
    };
    use build_html::{Html, HtmlContainer, HtmlPage, Table};
    use either::Either;
    use qrcode::QrCode;

    use super::{Enqueue, GetQuery};
    use crate::Server;
    use lib::errors;

    pub(super) async fn do_get_sha512(
        Query(GetQuery { digest: d }): Query<GetQuery>,
        server: Data<Server>,
    ) -> errors::Result<impl Responder> {
        log::trace!("/sha512");
        let path = {
            let digest = server.digest.read().await;
            digest.get(&d).ok_or(StatusCode::NOT_FOUND)?.to_owned()
        };

        let filename = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(StatusCode::NOT_FOUND)?
            .to_string();
        let header = (
            "Content-Disposition",
            format!(r#"attachment; filename="{}""#, filename),
        );

        let bytes = tokio::fs::read(path)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

        Ok(HttpResponse::build(StatusCode::OK)
            .insert_header(header)
            .message_body(bytes)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
    }

    fn a_href(url: impl Display, desc: impl Display) -> String {
        format!(r#"<a href="{}">{}</a>"#, url, desc)
    }

    /// Convert a digest pair into HTML strings.
    async fn htmlize_digest_pair(
        server: &Server,
        (digest, path): (&String, &PathBuf),
    ) -> Option<[String; 3]> {
        // get the download HTML tag from the digest
        let download = a_href(
            server.file_url(Either::Left(digest.clone())).await?,
            path.file_name().unwrap().to_string_lossy(),
        );

        // get the QR HTML tag from the digest
        let qr = a_href(
            server.qr_url(Either::Left(digest.clone())).await?,
            "QR code",
        );

        // only first 10 chars are important
        const HASH_SHOW_CHARS: usize = 10;
        let digest = digest[..HASH_SHOW_CHARS].to_string();

        Some([digest, download, qr])
    }

    pub(super) async fn do_list_files(
        server: Data<Server>,
    ) -> errors::Result<impl Responder> {
        log::trace!(
            "Listing server, currently {} file(s).",
            server.digest.read().await.len()
        );

        let table = {
            let digest = server.digest.read().await;

            let mut table =
                Table::new().with_header_row(["digests", "file names", ""]);

            for pair in &*digest {
                table.add_body_row(
                    htmlize_digest_pair(&server, pair)
                        .await
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                )
            }

            table
        };

        static TITLE: &str = "QR Share: Files";
        let page = HtmlPage::new()
            .with_title(TITLE)
            .with_header(1, TITLE)
            // this seems to be mostly how nginx autoindex displays file
            // listings
            .with_preformatted(table.to_html_string());

        let response = HttpResponse::build(StatusCode::OK)
            .content_type(ContentType::html())
            .body(page.to_html_string());

        Ok(response)
    }

    pub(super) async fn do_enqueue_file(
        server: Data<Server>,
        Json(files): Json<Enqueue>,
    ) -> errors::Result<impl Responder> {
        server.enqueue(files).await;
        Arc::clone(&server).process_digest().await?;

        Ok("Files successfully enqueued.\n")
    }

    /// Serve a file at `path` as a response, or 404 status if failed.
    pub(super) async fn serve_file_at(
        path: &Path,
    ) -> errors::Result<impl Responder> {
        log::info!("Serving file: {}", path.display());
        if let Ok(file) = NamedFile::open(path) {
            Ok(file)
        } else {
            log::error!("Cannot serve file: {}", path.display());
            Err(StatusCode::NOT_FOUND.into())
        }
    }

    pub(super) async fn do_show_qr(
        server: Data<Server>,
        Query(GetQuery { digest }): Query<GetQuery>,
    ) -> errors::Result<impl Responder> {
        let scheme = "http";
        let host = server.bind.primary_host();
        let port = server.bind.port();
        let method = "sha512";

        let url =
            format!("{}://{}:{}/{}/?h={}", scheme, host, port, method, digest);
        log::info!("Showing QR code for {}", url);

        let qr = QrCode::new(url)?;

        Ok(HttpResponse::Ok()
            .content_type(ContentType(mime::IMAGE_SVG))
            .message_body(qr.render::<qrcode::render::svg::Color>().build()))
    }
}
