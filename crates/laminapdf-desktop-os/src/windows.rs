//! Windows: the Shell's recent documents and the Jump List of this app's `AppUserModelID`.
#![expect(unsafe_code, reason = "the Shell's C and COM functions")]

use std::cell::Cell;
use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx};
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

thread_local! {
    /// Whether this thread has joined COM's single-threaded apartment.
    static COM: Cell<bool> = const { Cell::new(false) };
}

/// Joins COM's single-threaded apartment on this thread, once. The window system usually has
/// already (OLE drag and drop), which answers `S_FALSE`; a thread already in the multithreaded
/// apartment answers `RPC_E_CHANGED_MODE`, and the Shell's objects work from there too. The
/// apartment lasts as long as the thread, which is the app's main thread.
fn join_com() {
    if COM.get() {
        return;
    }
    // SAFETY: no reserved pointer; called on the thread that uses the objects, never torn down
    // while it runs.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    COM.set(true);
}

/// This app's Jump List lists: `IApplicationDestinations` scoped to `app_id`.
fn destinations(app_id: &str) -> Result<IApplicationDestinations, OsError> {
    join_com();
    // SAFETY: the class ID is the Shell's, and this thread is in a COM apartment (`join_com`).
    let destinations: IApplicationDestinations =
        unsafe { CoCreateInstance(&ApplicationDestinations, None, CLSCTX_INPROC_SERVER) }.map_err(|e| refused(&e))?;
    let id = HSTRING::from(app_id);
    // SAFETY: `id` is a NUL-terminated wide string that outlives the call.
    unsafe { destinations.SetAppID(&id) }.map_err(|e| refused(&e))?;
    Ok(destinations)
}

pub(crate) fn forget_recent(app_id: &str, path: &Path) -> Result<(), OsError> {
    let destinations = destinations(app_id)?;
    // The Shell item below needs COM too; `destinations` joined it.
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

/// `HKLM\Software\Policies\Lamina`'s update values.
pub(crate) fn managed_updates() -> crate::ManagedUpdates {
    const KEY: &str = r"Software\Policies\Lamina";
    crate::ManagedUpdates {
        channel: registry_string(KEY, "UpdateChannel"),
        disable: registry_dword(KEY, "DisableUpdates").map(|value| value != 0),
    }
}

/// `HKLM\Software\Lamina\InstallFlavour`, which only D6d's MSI writes.
pub(crate) fn install_flavour() -> Option<String> {
    registry_string(r"Software\Lamina", "InstallFlavour")
}

/// A `REG_SZ` value under `HKEY_LOCAL_MACHINE`; `None` when it is missing or of another type.
fn registry_string(key: &str, value: &str) -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    let mut bytes: u32 = 0;
    // SAFETY: the key and value names are NUL-terminated wide strings that outlive the call; with
    // no buffer, the call only writes the size it needs to `bytes`.
    let sized = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &key,
            &value,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&raw mut bytes),
        )
    };
    if sized.is_err() || bytes == 0 {
        return None;
    }
    let mut buffer = vec![0_u16; (bytes as usize).div_ceil(2)];
    // SAFETY: `buffer` holds `bytes` bytes, the size the first call asked for, and outlives the call.
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &key,
            &value,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&raw mut bytes),
        )
    };
    if read.is_err() {
        return None;
    }
    // The count includes the terminating NUL, which `RRF_RT_REG_SZ` guarantees.
    let text = buffer.split(|&unit| unit == 0).next().unwrap_or_default();
    String::from_utf16(text).ok()
}

/// A `REG_DWORD` value under `HKEY_LOCAL_MACHINE`; `None` when it is missing or of another type.
fn registry_dword(key: &str, value: &str) -> Option<u32> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    let mut data: u32 = 0;
    let mut bytes = u32::try_from(size_of::<u32>()).unwrap_or(4);
    // SAFETY: `data` is a 4-byte buffer, as `bytes` says, and outlives the call; the names are
    // NUL-terminated wide strings.
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &key,
            &value,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&raw mut bytes),
        )
    };
    read.is_ok().then_some(data)
}
