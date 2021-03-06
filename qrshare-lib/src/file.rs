/// Shared API
pub mod shared {
    use std::fs::FileType;

    /// Check whether a file type does not represent a single-read file.
    #[cfg(target_family = "unix")]
    pub fn is_multiread_md(ft: FileType) -> bool {
        use std::os::unix::fs::FileTypeExt;
        !ft.is_fifo() && !ft.is_socket()
    }

    /// Check whether a file type does not represent a single-read file.
    #[cfg(target_os = "wasi")]
    pub fn is_multiread_md(ft: FileType) -> bool {
        use std::os::wasi::fs::FileTypeExt;
        !ft.is_socket()
    }

    /// Check whether a file type does not represent a single-read file.
    #[cfg(windows)]
    pub fn is_multiread_md(_: FileType) -> bool {
        true
    }
}

/// Synchronous API
pub mod sync {
    pub use std::fs::{canonicalize, File};

    use super::shared::is_multiread_md;

    /// Check whether a file is a multi-read file.
    pub fn is_multiread_file(file: &File) -> bool {
        file.metadata()
            .map(|md| md.file_type())
            .map_or(false, is_multiread_md)
    }
}

/// Asynchronous API
pub mod asy {
    pub use tokio::fs::{canonicalize, File};

    use super::shared::is_multiread_md;

    /// Check whether a file is a multi-read file.
    pub async fn is_multiread_file(file: &File) -> bool {
        file.metadata()
            .await
            .map(|md| md.file_type())
            .map_or(false, is_multiread_md)
    }
}
