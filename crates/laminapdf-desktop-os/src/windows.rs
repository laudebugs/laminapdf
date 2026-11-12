//! Windows: the Shell's recent documents and the Jump List of this app's `AppUserModelID`.
#![expect(unsafe_code, reason = "the Shell's C and COM functions")]

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{
    ApplicationDestinations, IApplicationDestinations, IShellItem, SHARD_PATHW, SHAddToRecentDocs,
    SHCreateItemFromParsingName, SetCurrentProcessExplicitAppUserModelID,
};
use windows::core::{HSTRING, PCWSTR};

use crate::OsError;

fn refused(error: &windows::core::Error) -> OsError {
    OsError::Refused(error.message())
}

pub(crate) fn set_app_id(app_id: &str) -> Result<(), OsError> {
    let id = HSTRING::from(app_id);
    // SAFETY: `id` is a NUL-terminated wide string that outlives the call.
    unsafe { SetCurrentProcessExplicitAppUserModelID(&id) }.map_err(|e| refused(&e))
}

#[expect(clippy::unnecessary_wraps, reason = "every OS's version has the same signature")]
pub(crate) fn note_recent(path: &Path) -> Result<(), OsError> {
    let path = HSTRING::from(path.as_os_str());
    let wide = PCWSTR(path.as_ptr());
    // SAFETY: with SHARD_PATHW, `pv` is a NUL-terminated wide path, which `path` owns for the
    // call. It is never null: a null `pv` would clear every app's Recent Items.
    unsafe { SHAddToRecentDocs(SHARD_PATHW.0.cast_unsigned(), Some(wide.as_ptr().cast())) };
    Ok(())
}

/// This app's Jump List lists: `IApplicationDestinations` scoped to `app_id`.
fn destinations(app_id: &str) -> Result<IApplicationDestinations, OsError> {
    // SAFETY: the class ID is the Shell's, and COM is initialised on the calling (main) thread by
    // the window system before any window opens.
    let destinations: IApplicationDestinations =
        unsafe { CoCreateInstance(&ApplicationDestinations, None, CLSCTX_INPROC_SERVER) }.map_err(|e| refused(&e))?;
    let id = HSTRING::from(app_id);
    // SAFETY: `id` is a NUL-terminated wide string that outlives the call.
    unsafe { destinations.SetAppID(&id) }.map_err(|e| refused(&e))?;
    Ok(destinations)
}

pub(crate) fn forget_recent(app_id: &str, path: &Path) -> Result<(), OsError> {
    let destinations = destinations(app_id)?;
    let path = HSTRING::from(path.as_os_str());
    // SAFETY: `path` is a NUL-terminated wide string that outlives the call; no bind context.
    let Ok(item) = (unsafe { SHCreateItemFromParsingName::<_, _, IShellItem>(&path, None) }) else {
        // The file is gone, so the Shell can't name it; the Jump List drops missing files itself.
        return Ok(());
    };
    // SAFETY: `item` is a live shell item; the call only reads it.
    unsafe { destinations.RemoveDestination(&item) }.map_err(|e| refused(&e))
}

pub(crate) fn clear_recent(app_id: &str, _paths: &[PathBuf]) -> Result<(), OsError> {
    let destinations = destinations(app_id)?;
    // SAFETY: a live interface; the call takes no arguments and touches only this app ID's lists.
    unsafe { destinations.RemoveAllDestinations() }.map_err(|e| refused(&e))
}
