//! Putting a save's bytes where the document lives (E1 plan §7): appended to the file in place, or
//! a whole new file written beside it and renamed over it. Either way, a write that fails leaves
//! the file as it was. In the Mac App Store build the app may write only the files the user chose,
//! not new ones beside them, so the new file is written in macOS's replacement folder on the same
//! volume instead (ADR 0014 §11).
//!
//! `fill` writes the save's output, which the engine hands over in parts, so no step holds a whole
//! file in memory.

use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Seek as _, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// The error that stops a write because the file isn't the one the save began with: another app
/// changed it meanwhile. `io::Error::other(Changed)` carries it, so the caller can tell it apart.
#[derive(Debug, thiserror::Error)]
#[error("another app changed the file while it saved")]
pub(crate) struct Changed;

/// Whether `error` is [`Changed`].
pub(crate) fn is_changed(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(<dyn std::error::Error + Send + Sync>::is::<Changed>)
}

/// Writes the save's output into the writer it is given.
pub(crate) type Fill<'a> = dyn FnMut(&mut dyn Write) -> io::Result<()> + 'a;

/// Appends the output to `path`, which must still be `at` bytes long: an incremental update. A
/// failure cuts the file back to `at` bytes.
///
/// # Errors
/// The OS's error when the file can't be opened for writing (read-only, gone) or written (the
/// disk is full); `fill`'s own; [`Changed`] for a file whose length isn't `at`.
pub(crate) fn append(path: &Path, at: u64, fill: &mut Fill<'_>) -> io::Result<()> {
    let mut target = OpenOptions::new().write(true).open(path)?;
    if target.metadata()?.len() != at {
        return Err(io::Error::other(Changed));
    }
    let written = (|| {
        target.seek(SeekFrom::Start(at))?;
        let mut out = BufWriter::new(&mut target);
        fill(&mut out)?;
        out.flush()?;
        drop(out);
        target.sync_all()
    })();
    if let Err(error) = written {
        // Whatever the failed write left after the old end goes; the file is as it was.
        if let Err(cut) = target.set_len(at).and_then(|()| target.sync_all()) {
            tracing::error!("a failed save could not cut the file back: {cut}");
        }
        return Err(error);
    }
    Ok(())
}

/// Writes a new file at `path` in place of the old one: the output goes to a temporary file
/// beside it, which is renamed over `path` once it is complete, so the old file stays whole until
/// then. The new file keeps the old one's permissions. A read-only file is refused, as appending
/// to it would be, although its folder may allow the rename. `check` runs just before the rename:
/// its error stops it, and the old file stays.
///
/// # Errors
/// `PermissionDenied` for a read-only file; the OS's errors; `fill`'s and `check`'s own.
pub(crate) fn replace(path: &Path, fill: &mut Fill<'_>, check: &mut dyn FnMut() -> io::Result<()>) -> io::Result<()> {
    let permissions = fs::metadata(path)?.permissions();
    if permissions.readonly() {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    write_beside(path, Some(permissions), None, fill, check)
}

/// Writes a new file at `path` (Save as): `base`'s bytes followed by the output when the output
/// is an incremental update, or the output alone. A file already at `path` is replaced only once
/// the new one is complete.
///
/// # Errors
/// The OS's errors; `fill`'s own.
pub(crate) fn create(path: &Path, base: Option<&Path>, fill: &mut Fill<'_>) -> io::Result<()> {
    write_beside(path, None, base, fill, &mut || Ok(()))
}

/// Writes `base` (when given) and the output to a temporary file in `path`'s folder, then, once
/// `check` agrees, renames it over `path`. The temporary file is removed on any failure.
fn write_beside(
    path: &Path,
    permissions: Option<fs::Permissions>,
    base: Option<&Path>,
    fill: &mut Fill<'_>,
    check: &mut dyn FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let (temp, folder) = temporary_for(path)?;
    let written = (|| {
        let mut output = OpenOptions::new().write(true).create_new(true).open(&temp)?;
        {
            let mut out = BufWriter::new(&mut output);
            if let Some(base) = base {
                io::copy(&mut laminapdf_desktop_os::open_shared(base)?, &mut out)?;
            }
            fill(&mut out)?;
            out.flush()?;
        }
        output.sync_all()?;
        drop(output);
        if let Some(permissions) = permissions {
            fs::set_permissions(&temp, permissions)?;
        }
        check()?;
        fs::rename(&temp, path)?;
        sync_folder(path);
        Ok(())
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    if let Some(folder) = folder {
        let _ = fs::remove_dir_all(folder);
    }
    written
}

/// A name for a temporary file for `path` that no other save of this process uses: beside it, or
/// in the Mac App Store build in a replacement folder of its own, which is returned too for the
/// caller to remove.
fn temporary_for(path: &Path) -> io::Result<(PathBuf, Option<PathBuf>)> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
        .to_string_lossy();
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let temp = format!(".{name}.{}-{n}.lamina-save", std::process::id());
    if cfg!(feature = "app-store") {
        let folder = laminapdf_desktop_os::replacement_folder(path)?;
        return Ok((folder.join(temp), Some(folder)));
    }
    let folder = path
        .parent()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    Ok((folder.join(temp), None))
}

/// Makes the rename itself durable where the OS needs the folder synced for that (Unix). A
/// failure is only logged: the file is complete either way.
fn sync_folder(path: &Path) {
    #[cfg(unix)]
    if let Some(folder) = path.parent()
        && let Err(error) = fs::File::open(folder).and_then(|folder| folder.sync_all())
    {
        tracing::warn!("could not sync a saved file's folder: {error}");
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests;
