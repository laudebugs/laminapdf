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
