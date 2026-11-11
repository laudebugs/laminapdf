//! The desktop shell's error: what a command reports to the UI and what `run` returns.

use serde::Serialize;

/// Why something in the shell failed. Serialised for the UI as `{ "kind", "message" }`, so it
/// can map a failure to its own words without parsing text.
#[derive(Debug, thiserror::Error)]
pub enum DesktopError {
    /// Tauri itself failed (building the app, a window, the event loop).
    #[error("the desktop shell failed: {0}")]
    Tauri(#[from] tauri::Error),
    /// The engine process refused or failed a request; `kind` is the engine's reason
    /// (`wrong-password`, `invalid`, `engine-restarted` …).
    #[error("{message}")]
    Engine {
        /// The stable name the UI matches on.
        kind: &'static str,
        /// What happened, in words for the log. Never a path or document content.
        message: String,
    },
    /// A file couldn't be opened. Rust's I/O errors carry no path.
    #[error("the file could not be opened: {0}")]
    Io(#[from] std::io::Error),
    /// The window named a document it doesn't have open.
    #[error("no such document in this window")]
    UnknownDocument,
    /// A background task of the shell failed (the file dialog's thread).
    #[error("the desktop shell failed: {0}")]
    Internal(String),
}

impl DesktopError {
    /// The stable name the UI matches on.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Tauri(_) => "tauri",
            Self::Engine { kind, .. } => kind,
            Self::Io(_) => "io",
            Self::UnknownDocument => "unknown-document",
            Self::Internal(_) => "internal",
        }
    }
}

impl Serialize for DesktopError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        let mut s = serializer.serialize_struct("DesktopError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
