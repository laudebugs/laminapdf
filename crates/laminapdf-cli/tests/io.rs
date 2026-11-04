//! `ProcessIo` on the real file system: a file is replaced only with `--force` and through a
//! temporary file, and failed reads and writes are the CLI's I/O errors.
#![expect(clippy::unwrap_used, reason = "test helpers: a panic is a test failure")]

use std::fs;
use std::path::{Path, PathBuf};

use laminapdf_cli::io::{Io as _, ProcessIo};

/// An empty folder of this test's own.
fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("laminapdf-cli-io")
        .join(name);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn a_file_should_be_written_and_replaced_only_with_force() {
    let path = folder("replace").join("out.pdf");
    let out = path.to_str().unwrap();
    let mut io = ProcessIo::default();
    io.write(out, b"one", false).unwrap();
    let refused = io.write(out, b"two", false).unwrap_err();
    assert_eq!(refused.diagnostic().code, "overwrite-refused");
    io.write(out, b"two", true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"two");
    // Only the output is left: the temporary file was renamed over it.
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn a_missing_folder_should_be_output_unwritable() {
    let path = folder("missing").join("no").join("out.pdf");
    let error = ProcessIo::default()
        .write(path.to_str().unwrap(), b"x", false)
        .unwrap_err();
    assert_eq!(
        (error.diagnostic().code.as_str(), error.data()["reason"].as_str()),
        ("output-unwritable", Some("NotFound"))
    );
}

#[test]
fn a_missing_input_should_be_input_unreadable() {
    let error = ProcessIo::default().read("/no/such/file.pdf").unwrap_err();
    assert_eq!(error.diagnostic().code, "input-unreadable");
    assert_eq!(error.data()["input"], "/no/such/file.pdf");
    assert!(
        !error.to_string().contains("file.pdf"),
        "the message never quotes the caller"
    );
}

#[cfg(unix)]
#[test]
fn a_replaced_file_should_keep_its_permissions() {
    use std::os::unix::fs::PermissionsExt as _;
    let path = folder("permissions").join("private.pdf");
    fs::write(&path, b"old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    ProcessIo::default()
        .write(path.to_str().unwrap(), b"new", true)
        .unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!((fs::read(&path).unwrap(), mode), (b"new".to_vec(), 0o600));
}

#[cfg(unix)]
#[test]
fn a_symlink_at_the_temporary_name_should_never_be_followed() {
    let dir = folder("symlink");
    let victim = dir.join("victim.txt");
    fs::write(&victim, b"keep").unwrap();
    let planted = dir.join(format!(".out.pdf.laminapdf-{}-0.tmp", std::process::id()));
    std::os::unix::fs::symlink(&victim, &planted).unwrap();
    let out = dir.join("out.pdf");
    ProcessIo::default()
        .write(out.to_str().unwrap(), b"pdf", false)
        .unwrap();
    assert_eq!(
        (fs::read(&victim).unwrap(), fs::read(&out).unwrap()),
        (b"keep".to_vec(), b"pdf".to_vec())
    );
}
