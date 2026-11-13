//! Linux: GTK's `RecentManager` (`recently-used.xbel`), shared by every app.

use std::path::{Path, PathBuf};

use gtk::RecentManager;
use gtk::prelude::RecentManagerExt as _;

use crate::OsError;

fn manager() -> Result<RecentManager, OsError> {
    if !gtk::is_initialized_main_thread() {
        return Err(OsError::NotMainThread);
    }
    RecentManager::default().ok_or_else(|| OsError::Refused("no recent-files manager".into()))
}

fn uri(path: &Path) -> Result<gtk::glib::GString, OsError> {
    gtk::glib::filename_to_uri(path, None).map_err(|error| OsError::Refused(error.message().to_owned()))
}

pub(crate) fn note_recent(path: &Path) -> Result<(), OsError> {
    if manager()?.add_item(&uri(path)?) {
        Ok(())
    } else {
        Err(OsError::Refused("the file wasn't added to the recent list".into()))
    }
}

pub(crate) fn forget_recent(_app_id: &str, path: &Path) -> Result<(), OsError> {
    // An entry already gone (removed in a file manager) is fine.
    let _ = manager()?.remove_item(&uri(path)?);
    Ok(())
}

pub(crate) fn clear_recent(app_id: &str, paths: &[PathBuf]) -> Result<(), OsError> {
    for path in paths {
        forget_recent(app_id, path)?;
    }
    Ok(())
}
