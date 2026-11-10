//! The desktop shell's error: what a command reports to the UI and what `run` returns.

use serde::Serialize;

/// Why something in the shell failed. Serialised for the UI as `{ "kind", "message" }`, so it
/// can map a failure to its own words without parsing text.
#[derive(Debug, thiserror::Error)]
pub enum DesktopError {
    /// Tauri itself failed (building the app, a window, the event loop).
    #[error("the desktop shell failed: {0}")]
    Tauri(#[from] tauri::Error),
}

impl DesktopError {
    /// The stable name the UI matches on.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Tauri(_) => "tauri",
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
