//! Error types for `aspid-core`.

use std::path::PathBuf;

/// Result alias used throughout the core.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type for all core operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The Hollow Knight installation could not be located automatically.
    #[error("could not locate a Hollow Knight installation; set the game path manually")]
    GameNotFound,

    /// A path was expected to point at a valid game install but did not.
    #[error("invalid Hollow Knight installation at {path}: {reason}")]
    InvalidInstall {
        /// The path that failed validation.
        path: PathBuf,
        /// Why validation failed.
        reason: String,
    },

    /// A required platform directory (config/data/cache) could not be resolved.
    #[error("could not resolve the {0} directory for this platform")]
    MissingPlatformDir(&'static str),

    /// A downloaded artifact failed SHA-256 verification.
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// The hash declared by ModLinks/ApiLinks.
        expected: String,
        /// The hash we computed from the downloaded bytes.
        actual: String,
    },

    /// A mod referenced a dependency that is not present in the catalog.
    #[error("unknown dependency `{0}`")]
    UnknownDependency(String),

    /// No download is available for the current platform.
    #[error("no download available for {what} on this platform")]
    NoDownloadForPlatform {
        /// What was being downloaded (a mod name, or the modding API).
        what: String,
    },

    /// An operation required the modding API but it is not installed.
    #[error("the modding API is not installed")]
    ApiNotInstalled,

    /// Failed to parse a ModLinks/ApiLinks document.
    #[error("failed to parse {what}: {source}")]
    Xml {
        /// Which document failed to parse.
        what: &'static str,
        /// The underlying parse error.
        #[source]
        source: quick_xml::DeError,
    },

    /// A network request failed.
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),

    /// A filesystem operation failed.
    #[error("io error at {path}: {source}")]
    Io {
        /// The path involved in the operation, when known.
        path: PathBuf,
        /// The underlying io error.
        #[source]
        source: std::io::Error,
    },

    /// A zip archive could not be extracted.
    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// (De)serialization of persisted config/state failed.
    #[error("config error: {0}")]
    Config(String),
}

impl Error {
    /// Helper to attach a path to a bare [`std::io::Error`].
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }
}
