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
        child.stdin.take().unwrap().write_all(bytes).unwrap();
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
