//! The crate's canonical error type.
//!
//! Follows M-ERRORS-CANONICAL-STRUCTS: a single situation-typed [`Error`] struct
//! with an internal [`ErrorKind`], `is_*` predicate helpers, and `Display` +
//! `std::error::Error` implementations. Callers should branch on the helpers rather
//! than match `ErrorKind` exhaustively, so new kinds aren't breaking changes.

use std::fmt;

/// Convenience alias for results from this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// An error from any KiCad access backend.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

/// The category of an [`Error`]. Non-exhaustive so adding variants is non-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Could not connect to a running KiCad instance over the IPC socket.
    NotConnected,
    /// KiCad is reachable but the requested operation is unsupported in this version.
    Unsupported,
    /// A file on disk could not be read or parsed.
    File,
    /// The `kicad-cli` invocation failed (non-zero exit or spawn failure).
    Cli,
    /// The backend required for this call was not compiled in (missing feature).
    BackendDisabled,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub(crate) fn with_source(
        kind: ErrorKind,
        source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            kind,
            source: Some(source.into()),
        }
    }

    /// Returns the category of this error.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// True if this error means no live KiCad instance was reachable.
    #[must_use]
    pub fn is_not_connected(&self) -> bool {
        self.kind == ErrorKind::NotConnected
    }

    /// True if the needed backend feature was not enabled at compile time.
    #[must_use]
    pub fn is_backend_disabled(&self) -> bool {
        self.kind == ErrorKind::BackendDisabled
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self.kind {
            ErrorKind::NotConnected => "no running KiCad instance on the IPC socket",
            ErrorKind::Unsupported => "operation unsupported by this KiCad version",
            ErrorKind::File => "could not read or parse KiCad file",
            ErrorKind::Cli => "kicad-cli invocation failed",
            ErrorKind::BackendDisabled => "required backend feature not enabled",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}
