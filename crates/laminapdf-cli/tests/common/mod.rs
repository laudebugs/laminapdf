//! Running the `laminapdf` binary in tests: from the repository root (so fixture paths read as
//! `fixtures/…`), with standard input closed or given, and no password in the environment.
#![allow(
    dead_code,
    unreachable_pub,
    reason = "each test file uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

pub mod schemas;

/// A finished run.
#[derive(Debug)]
pub struct Run {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Run {
    pub fn stdout_text(&self) -> String {
        String::from_utf8(self.stdout.clone()).unwrap()
    }

    /// Standard output as one JSON document.
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.stdout).unwrap_or_else(|e| panic!("{e}: {}", self.stdout_text()))
    }

    /// Each line of standard error as JSON.
    pub fn stderr_json(&self) -> Vec<Value> {
        self.stderr
            .lines()
            .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
            .collect()
    }
}

/// The repository root.
pub fn root() -> PathBuf {
    laminapdf_test_fixtures::fixtures_dir().parent().unwrap().to_owned()
}

/// `laminapdf args…` with standard input closed.
pub fn laminapdf(args: &[&str]) -> Run {
    run(args, None, &[])
}

/// `laminapdf args…` with `stdin` piped in (closed when `None`) and `env` set.
pub fn run(args: &[&str], stdin: Option<&[u8]>, env: &[(&str, &str)]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_laminapdf"));
    command
        .args(args)
        .current_dir(root())
        .env_remove("LAMINA_PDF_PASSWORD")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .envs(env.iter().copied())
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(bytes) = stdin {
        // A command that refuses its arguments can exit before it reads standard input; the
        // write then fails with a broken pipe, and the exit code and stderr below still say
        // what happened. Any other write error is a test failure.
        if let Err(e) = child.stdin.take().unwrap().write_all(bytes) {
            assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe, "{e}");
        }
    }
    let output = child.wait_with_output().unwrap();
    Run {
        code: output.status.code().unwrap(),
        stdout: output.stdout,
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// An empty folder of the test's own.
pub fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("laminapdf-cli").join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// A path as an argument.
pub fn arg(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// A one-page file drawing `text` in Helvetica 12 at (72, 700), then, with `cover`, a black
/// box over all of it (todo T2b's covered page).
pub fn page_with_text(text: &str, cover: bool) -> Vec<u8> {
    let mut content = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET");
    if cover {
        content.push_str(" 0 g 70 694 400 16 re f");
    }
    let stream = laminapdf_test_fixtures::stream_body("", content.as_bytes());
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        &stream,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ];
    laminapdf_test_fixtures::classic(&objects, "<< /Size 6 /Root 1 0 R >>")
}

/// `base` with an incremental update (ISO 32000-2 §7.5.6) that replaces object `num` with
/// `body`, as an editor's plain save does.
pub fn with_update(base: &[u8], num: u32, body: &[u8]) -> Vec<u8> {
    laminapdf_test_fixtures::update(base, &[(num, body)], "/Size 6 /Root 1 0 R")
}
