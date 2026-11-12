//! The calls Lamina PDF's desktop reader makes into the OS's shell (ADR 0008): its
//! recent-documents lists and, on Windows, the app ID they belong to.
//!
//! - **macOS:** `NSDocumentController`'s recent documents, which the Dock menu and File ▸ Open
//!   Recent show. `AppKit` keeps one list per app, so clearing it clears only Lamina PDF's; it has
//!   no call to remove one entry.
//! - **Windows:** `SHAddToRecentDocs` with a path (`SHARD_PATHW`), which feeds the Jump List's
//!   Recent category of the process's app ID ([`set_app_id`]). Clearing and removing go through
//!   `IApplicationDestinations`, scoped to that app ID, so they touch only Lamina PDF's Jump List.
//!   Never `SHAddToRecentDocs` with a null `pv`: that empties every app's Recent Items.
//! - **Linux:** GTK's `RecentManager` (`recently-used.xbel`), which file managers show; clearing
//!   removes only the entries this app added.
//!
//! Every call must run on the app's main thread (`AppKit`'s and GTK's rule; on Windows the
//! calls join that thread's COM apartment). Elsewhere the calls do nothing on macOS and fail on
//! Linux.

use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
use macos as os;
#[cfg(windows)]
use windows as os;

/// Why the OS refused a call. Never carries a path.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OsError {
    /// The call ran off the main thread.
    #[error("the OS's recent list can only be changed from the main thread")]
    NotMainThread,
    /// The OS refused the call.
    #[error("the OS refused the call: {0}")]
    Refused(String),
}

/// Adds `path` to the OS's recent documents for this app.
///
/// # Errors
/// [`OsError`] when the OS refused, or off the main thread.
pub fn note_recent(path: &Path) -> Result<(), OsError> {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    return os::note_recent(path);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = path;
        Ok(())
    }
}

/// Removes `path` from the OS's recent documents for this app, where the OS can remove one entry
/// (Windows, Linux); macOS keeps it until the list is cleared.
///
/// # Errors
/// [`OsError`] when the OS refused, or off the main thread.
pub fn forget_recent(app_id: &str, path: &Path) -> Result<(), OsError> {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    return os::forget_recent(app_id, path);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = (app_id, path);
        Ok(())
    }
}

/// Clears this app's recent documents: its whole list on macOS and Windows (the Jump List of
/// `app_id`), the entries for `paths` on Linux, where the list is shared by every app.
///
/// # Errors
/// [`OsError`] when the OS refused, or off the main thread.
pub fn clear_recent(app_id: &str, paths: &[PathBuf]) -> Result<(), OsError> {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    return os::clear_recent(app_id, paths);
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = (app_id, paths);
        Ok(())
    }
}

/// Windows: names this process's app (its `AppUserModelID`), which must match the installer's
/// shortcuts so the taskbar, its Jump List and [`clear_recent`] mean the same list. Call it once at
/// start-up, before any window opens. Elsewhere it does nothing.
///
/// # Errors
/// [`OsError::Refused`] when Windows refused the ID.
pub fn set_app_id(app_id: &str) -> Result<(), OsError> {
    #[cfg(windows)]
    return os::set_app_id(app_id);
    #[cfg(not(windows))]
    {
        let _ = app_id;
        Ok(())
    }
}
