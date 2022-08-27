/// Generate the QR code from a file
pub mod gen {
    use std::{
        fmt::{self, Display, Formatter},
        io::ErrorKind,
        net::SocketAddr,
        path::PathBuf,
    };

    use http::Uri;
    use image::Luma;
    use qrcode::{render::svg, QrCode};
    use tempfile::TempDir;
    use tokio::{fs::File, io::AsyncWriteExt};

    use crate::{
        config::ImageOptions,
        errors,
        net::{get_first_net, is_global_4},
    };

    pub use self::svg::Color as SvgColor;

    /// Which file type to render.
    #[derive(Debug, Clone, Copy)]
    pub enum QrFileType {
        Png,
        Svg,
    }

    impl Display for QrFileType {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                QrFileType::Png => write!(f, "png"),
                QrFileType::Svg => write!(f, "svg"),
            }
        }
    }

    /// Generate a QR code file from a digest.  The lifetime is used for working
    /// with [`tempfile`] crate whose security promise states that the temporary
    /// directory is removed when the [`tempfile::TempDir`] object goes
    /// out-of-scope.
    pub async fn gen_qr<'dir>(
        addr: SocketAddr,
        digest: &str,
        method: &str, // sha512
        scheme: &str, // http
        ft: ImageOptions,
        dir: &'dir TempDir,
    ) -> errors::Result<PathBuf> {
        let host = addr.ip();
        let host = if is_global_4(&host) {
            host
        } else {
            get_first_net(is_global_4).ok_or(errors::Error::NoGlobalIpv4)?
        };
        let port = addr.port();

        // construct and validate URL
        let url =
            format!("{}://{}:{}/{}/?h={}", scheme, host, port, method, digest);
        let _: Uri =
            url.parse().map_err(|_| errors::Error::Uri(url.clone()))?;

        let path = dir.path().join(format!("{}_{}.{}", method, "qrshare", ft));

        let qr = QrCode::new(url)?;
        match ft {
            ImageOptions::None => {
                return Err(errors::Error::IO(ErrorKind::Other))
            }
            ImageOptions::Png => qr.render::<Luma<u8>>().build().save(&path)?,
            ImageOptions::Svg => {
                let mut file = File::create(&path).await?;
                file.write_all(qr.render::<svg::Color>().build().as_bytes())
                    .await?;
                file.flush().await?;
            }
        };

        Ok(path)
    }
}

/// Show the QR code
pub mod show {
    use std::path::Path;

    use crate::errors;

    /// Show a QR code for the path.  See [`open`] crate for further details.
    pub async fn qr_show(qr_path: impl AsRef<Path>) -> errors::Result<()> {
        Ok(open::that(qr_path.as_ref().as_os_str())?)
    }
}
