//! Putting a save's bytes where the document lives (E1 plan §7): appended to the file in place, or
//! a whole new file written beside it and renamed over it. Either way, a write that fails leaves
//! the file as it was.
//!
//! `fill` writes the save's output, which the engine hands over in parts, so no step holds a whole
//! file in memory.

use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Seek as _, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Writes the save's output into the writer it is given.
pub(crate) type Fill<'a> = dyn FnMut(&mut dyn Write) -> io::Result<()> + 'a;

/// Appends the output to `path`, which must still be `at` bytes long: an incremental update. A
/// failure cuts the file back to `at` bytes.
///
/// # Errors
/// The OS's error when the file can't be opened for writing (read-only, gone) or written (the
/// disk is full); `fill`'s own; `Other` for a file whose length isn't `at`.
pub(crate) fn append(path: &Path, at: u64, fill: &mut Fill<'_>) -> io::Result<()> {
    let mut target = OpenOptions::new().write(true).open(path)?;
    if target.metadata()?.len() != at {
        return Err(io::Error::other("the file's length changed since it opened"));
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
    let temp = temporary_beside(path)?;
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
    written
}

/// A name for a temporary file beside `path` that no other save of this process uses.
fn temporary_beside(path: &Path) -> io::Result<PathBuf> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let folder = path
        .parent()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
        .to_string_lossy();
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Ok(folder.join(format!(".{name}.{}-{n}.lamina-save", std::process::id())))
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
