use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CapstoneError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("could not decode audio: {0}")]
    Decode(#[from] symphonia::core::errors::Error),

    #[error("could not write wav: {0}")]
    Wav(#[from] hound::Error),

    #[error("could not encode audio: {0}")]
    Encode(String),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("not found: {0}")]
    NotFound(String),

    /// A run the user stopped. Separate from the error arms so the queue runner
    /// can tell "the user pressed cancel" apart from "this file is broken" and
    /// leave the row's own status alone.
    #[error("cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl Serialize for CapstoneError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CapstoneError>;

impl From<anyhow::Error> for CapstoneError {
    fn from(e: anyhow::Error) -> Self {
        CapstoneError::Other(e.to_string())
    }
}

/// aifc's own error type derives Debug and nothing else: no Display, so no
/// std::error::Error, so it cannot ride a bare `?` the way every other
/// encoder's error can. Hence the `{:?}` and one From impl here, rather than a
/// map_err at each AIFF call site.
impl From<aifc::AifcError> for CapstoneError {
    fn from(e: aifc::AifcError) -> Self {
        CapstoneError::Encode(format!("aiff: {:?}", e))
    }
}
