//! One error type for every command: the UI shows its text to the player.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }

    /// True when the operation failed for lack of rights (e.g. a game folder below Program Files).
    pub fn is_access_denied(&self) -> bool {
        matches!(self, Error::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied)
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Adds the path to an I/O error, as the decomp's setup does ("Cannot write <path>: ...").
pub trait Context<T> {
    fn context(self, what: &str, path: &std::path::Path) -> Result<T>;
}

impl<T> Context<T> for std::io::Result<T> {
    fn context(self, what: &str, path: &std::path::Path) -> Result<T> {
        self.map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // Keep the kind so callers can retry with administrator rights.
                Error::Io(std::io::Error::new(e.kind(), format!("{what} {}: {e}", path.display())))
            } else {
                Error::msg(format!("{what} {}: {e}", path.display()))
            }
        })
    }
}
