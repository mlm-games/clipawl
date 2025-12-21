//! Error types for clipawl.

use std::error::Error as StdError;

/// Clipboard operation error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Clipboard is not supported on this platform or in this environment.
    #[error("clipboard not supported on this platform")]
    NotSupported,

    /// Permission denied (web: user activation required, Android: no focus).
    #[error("clipboard permission denied: {0}")]
    PermissionDenied(&'static str),

    /// Clipboard is unavailable (empty, no text representation, etc.).
    #[error("clipboard unavailable: {0}")]
    Unavailable(&'static str),

    /// Platform-specific error with context.
    #[error("{context}: {source}")]
    Platform {
        /// Context describing where the error occurred.
        context: &'static str,
        /// The underlying error.
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },
}

impl Error {
    /// Create a platform error with context.
    pub(crate) fn platform<E>(context: &'static str, err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Platform {
            context,
            source: Box::new(err),
        }
    }
}
