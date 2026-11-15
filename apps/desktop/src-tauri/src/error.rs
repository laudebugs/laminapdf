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
    /// A tab was to move to a window that isn't open (or to the one it is in).
    #[error("no such window")]
    UnknownWindow,
    /// The file is open already: its tab was shown instead of opening it again.
    #[error("the file is open already")]
    AlreadyOpen,
    /// An administrator, an MSI install or the package manager decides the app's updates.
    #[error("updates are managed for this install")]
    UpdatesManaged,
    /// The channel can't be chosen: Stable before its first release, or any switch on a nightly.
    #[error("that update channel isn't available")]
    ChannelUnavailable,
    /// Restart to update was asked for with no update downloaded.
    #[error("no update is ready to install")]
    UpdateNotReady,
    /// The window is printing already: one print per window at a time.
    #[error("this window is printing already")]
    AlreadyPrinting,
    /// The document's file changed on disk since it opened or was last saved: saving over it
    /// would lose the other app's change, so the window asks first.
    #[error("the file changed on disk since it opened")]
    FileChanged,
    /// Writing the document's file failed; the file is as it was. Rust's I/O errors carry no path.
    #[error("the file could not be written: {0}")]
    Write(std::io::Error),
    /// A background task of the shell failed (the file dialog's thread).
    #[error("the desktop shell failed: {0}")]
    Internal(String),
}

impl DesktopError {
    /// The stable name the UI matches on.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Tauri(_) => "tauri",
            Self::Engine { kind, .. } => kind,
            Self::Io(_) => "io",
            Self::UnknownDocument => "unknown-document",
            Self::UnknownWindow => "unknown-window",
            Self::AlreadyOpen => "already-open",
            Self::UpdatesManaged => "updates-managed",
            Self::ChannelUnavailable => "channel-unavailable",
            Self::UpdateNotReady => "update-not-ready",
            Self::AlreadyPrinting => "already-printing",
            Self::FileChanged => "file-changed",
            Self::Write(error) => match error.kind() {
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem => "write-refused",
                std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded => "disk-full",
                _ => "write-failed",
            },
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
