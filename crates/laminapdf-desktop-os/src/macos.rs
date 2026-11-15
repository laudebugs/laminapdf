//! macOS: `NSDocumentController`'s recent documents.
#![expect(unsafe_code, reason = "AppKit's clearRecentDocuments: takes an untyped sender")]

use std::path::{Path, PathBuf};

use objc2::MainThreadMarker;
use objc2_app_kit::NSDocumentController;
use objc2_foundation::{NSString, NSURL};

use crate::OsError;

fn controller() -> Result<objc2::rc::Retained<NSDocumentController>, OsError> {
    let main = MainThreadMarker::new().ok_or(OsError::NotMainThread)?;
    Ok(NSDocumentController::sharedDocumentController(main))
}

pub(crate) fn note_recent(path: &Path) -> Result<(), OsError> {
    // A path that isn't UTF-8 can't be an NSString without loss; AppKit would list another file.
    let Some(path) = path.to_str() else {
        return Ok(());
    };
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    controller()?.noteNewRecentDocumentURL(&url);
    Ok(())
}

/// `AppKit` has no call to remove one recent document.
#[expect(clippy::unnecessary_wraps, reason = "every OS's version has the same signature")]
pub(crate) fn forget_recent(_app_id: &str, _path: &Path) -> Result<(), OsError> {
    Ok(())
}

pub(crate) fn clear_recent(_app_id: &str, _paths: &[PathBuf]) -> Result<(), OsError> {
    let controller = controller()?;
    // SAFETY: `sender` may be nil (AppKit's action methods accept a nil sender); nothing else is
    // passed, and the call runs on the main thread (checked by `controller`).
    unsafe { controller.clearRecentDocuments(None) };
    Ok(())
}

/// The forced (managed) `UpdateChannel` and `DisableUpdates` of the app's preferences domain.
/// `NSUserDefaults` is thread-safe, so this runs on any thread.
pub(crate) fn managed_updates() -> crate::ManagedUpdates {
    use objc2_foundation::NSUserDefaults;
    let defaults = NSUserDefaults::standardUserDefaults();
    let forced = |key: &NSString| defaults.objectIsForcedForKey(key);
    let channel_key = NSString::from_str("UpdateChannel");
    let disable_key = NSString::from_str("DisableUpdates");
    crate::ManagedUpdates {
        channel: forced(&channel_key)
            .then(|| defaults.stringForKey(&channel_key).map(|s| s.to_string()))
            .flatten(),
        disable: forced(&disable_key).then(|| defaults.boolForKey(&disable_key)),
    }
}
