//! The writes against real files in a temporary folder: what each leaves when it works, and that
//! a failure leaves the file as it was, with no temporary file behind.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::{append, create, replace};

/// A temporary folder with `doc.pdf` holding `bytes`; removed when dropped.
struct Folder {
    dir: PathBuf,
    file: PathBuf,
}

impl Folder {
    fn new(name: &str, bytes: &[u8]) -> Self {
        let dir = std::env::temp_dir().join(format!("laminapdf-save-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("doc.pdf");
        std::fs::write(&file, bytes).unwrap();
        Self { dir, file }
    }

    fn read(&self) -> Vec<u8> {
        std::fs::read(&self.file).unwrap()
    }

    /// The names in the folder, sorted: only `doc.pdf` after a save, whether it worked or not.
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn set_read_only(&self, read_only: bool) {
        let mut permissions = std::fs::metadata(&self.file).unwrap().permissions();
        permissions.set_readonly(read_only);
        std::fs::set_permissions(&self.file, permissions).unwrap();
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        self.set_read_only(false);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A fill that writes `bytes`.
fn writes(bytes: &'static [u8]) -> impl FnMut(&mut dyn Write) -> io::Result<()> {
    move |out| out.write_all(bytes)
}

/// A fill that writes half its bytes, then fails as a full disk would.
fn fails_halfway(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"half a revis")?;
    out.flush()?;
    Err(io::Error::from(io::ErrorKind::StorageFull))
}

fn kind(result: io::Result<()>) -> Option<io::ErrorKind> {
    result.err().map(|error| error.kind())
}

#[test]
fn an_append_should_grow_the_file_by_the_revision_alone() {
    let folder = Folder::new("append", b"%PDF-1.7 base");
    append(&folder.file, 13, &mut writes(b" revision")).unwrap();
    assert_eq!(folder.read(), b"%PDF-1.7 base revision");
}

#[test]
fn a_failed_append_should_leave_the_file_as_it_was() {
    let folder = Folder::new("append-fails", b"%PDF-1.7 base");
    let result = append(&folder.file, 13, &mut fails_halfway);
    assert_eq!(
        (kind(result), folder.read()),
        (Some(io::ErrorKind::StorageFull), b"%PDF-1.7 base".to_vec())
    );
}

#[test]
fn an_append_to_a_file_of_another_length_should_be_refused() {
    let folder = Folder::new("append-length", b"%PDF-1.7 base, changed since");
    let result = append(&folder.file, 13, &mut writes(b" revision"));
    assert_eq!(
        (result.as_ref().is_err_and(super::is_changed), folder.read()),
        (true, b"%PDF-1.7 base, changed since".to_vec())
    );
}

#[test]
fn an_append_to_a_read_only_file_should_fail_and_leave_it_as_it_was() {
    let folder = Folder::new("append-read-only", b"%PDF-1.7 base");
    folder.set_read_only(true);
    let result = append(&folder.file, 13, &mut writes(b" revision"));
    assert!(result.is_err() && folder.read() == b"%PDF-1.7 base");
}

#[test]
fn a_replacement_should_write_the_new_file_and_leave_nothing_beside_it() {
    let folder = Folder::new("replace", b"%PDF-1.7 old");
    replace(&folder.file, &mut writes(b"%PDF-1.7 rewritten"), &mut || Ok(())).unwrap();
    assert_eq!(
        (folder.read(), folder.names()),
        (b"%PDF-1.7 rewritten".to_vec(), vec!["doc.pdf".to_owned()])
    );
}

#[test]
fn a_failed_replacement_should_keep_the_old_file_and_remove_the_temporary_one() {
    let folder = Folder::new("replace-fails", b"%PDF-1.7 old");
    let result = replace(&folder.file, &mut fails_halfway, &mut || Ok(()));
    assert_eq!(
        (kind(result), folder.read(), folder.names()),
        (
            Some(io::ErrorKind::StorageFull),
            b"%PDF-1.7 old".to_vec(),
            vec!["doc.pdf".to_owned()]
        )
    );
}

#[test]
fn a_read_only_file_should_not_be_replaced() {
    let folder = Folder::new("replace-read-only", b"%PDF-1.7 old");
    folder.set_read_only(true);
    let result = replace(&folder.file, &mut writes(b"%PDF-1.7 rewritten"), &mut || Ok(()));
    assert_eq!(
        (kind(result), folder.read()),
        (Some(io::ErrorKind::PermissionDenied), b"%PDF-1.7 old".to_vec())
    );
}

#[cfg(unix)]
#[test]
fn a_replacement_should_keep_the_files_permissions() {
    use std::os::unix::fs::PermissionsExt as _;
    let folder = Folder::new("replace-mode", b"%PDF-1.7 old");
    std::fs::set_permissions(&folder.file, std::fs::Permissions::from_mode(0o640)).unwrap();
    replace(&folder.file, &mut writes(b"%PDF-1.7 rewritten"), &mut || Ok(())).unwrap();
    let mode = std::fs::metadata(&folder.file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o640);
}

#[test]
fn a_new_file_should_hold_the_base_followed_by_the_revision() {
    let folder = Folder::new("create", b"%PDF-1.7 base");
    let copy = folder.dir.join("copy.pdf");
    create(&copy, Some(&folder.file), &mut writes(b" revision")).unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), b"%PDF-1.7 base revision");
}

#[test]
fn a_new_file_from_a_rewrite_should_hold_the_output_alone() {
    let folder = Folder::new("create-rewrite", b"%PDF-1.7 base");
    let copy = folder.dir.join("copy.pdf");
    create(&copy, None::<&Path>, &mut writes(b"%PDF-1.7 rewritten")).unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), b"%PDF-1.7 rewritten");
}

#[test]
fn a_replacement_whose_check_fails_should_keep_the_old_file_and_remove_the_temporary_one() {
    let folder = Folder::new("replace-checked", b"%PDF-1.7 old");
    let mut changed = || Err(io::Error::from(io::ErrorKind::WouldBlock));
    let result = replace(&folder.file, &mut writes(b"%PDF-1.7 rewritten"), &mut changed);
    assert_eq!(
        (kind(result), folder.read(), folder.names()),
        (
            Some(io::ErrorKind::WouldBlock),
            b"%PDF-1.7 old".to_vec(),
            vec!["doc.pdf".to_owned()]
        )
    );
}

/// The store build's way, which only App Sandbox picks at run time: the new file is written in a
/// replacement folder on the document's volume and renamed over the document.
#[cfg(target_os = "macos")]
#[test]
fn a_temporary_file_in_a_replacement_folder_should_rename_over_the_document() {
    let dir = std::env::temp_dir().join(format!("laminapdf-write-sandboxed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doc.pdf");
    std::fs::write(&path, b"old").unwrap();
    let (temp, folder) = super::temporary_for(&path, true).unwrap();
    std::fs::write(&temp, b"new").unwrap();
    std::fs::rename(&temp, &path).unwrap();
    let outside = folder.as_ref().is_some_and(|folder| !folder.starts_with(&dir));
    let _ = folder.map(std::fs::remove_dir_all);
    let read = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!((read, outside), (b"new".to_vec(), true));
}
