//! Where commands read their inputs and write their outputs ([ADR 0031] §4).
//!
//! Commands never touch the file system or the standard streams directly: they go through
//! [`Io`], so the front end runs them on the real process ([`ProcessIo`]) and tests or the MCP
//! adapter on something else. `-` names standard input or output.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use std::fs;
use std::io::{IsTerminal as _, Read as _, Write as _};
use std::path::Path;

use laminapdf::ErrorCode;

use crate::exit::CliError;

/// The name of standard input or standard output.
pub const STDIO: &str = "-";

/// Inputs and outputs, as commands see them.
pub trait Io {
    /// The bytes of `input`: a path, or `-` for standard input (at most once).
    ///
    /// # Errors
    ///
    /// `input-unreadable`; `invalid-command-line` for a second read of standard input.
    fn read(&mut self, input: &str) -> Result<Vec<u8>, CliError>;

    /// Writes the document `bytes` to `output`: a path, or `-` for standard output. A path
    /// that exists is replaced only with `force`, and through a temporary file in its folder,
    /// so a failed write leaves it as it was.
    ///
    /// # Errors
    ///
    /// `overwrite-refused`; `binary-output-to-terminal` when `output` is `-` and standard
    /// output is a terminal; `output-unwritable`.
    fn write(&mut self, output: &str, bytes: &[u8], force: bool) -> Result<(), CliError>;
}

/// The real process: the file system, standard input and standard output.
#[derive(Debug, Default)]
pub struct ProcessIo {
    stdin_read: bool,
}

impl Io for ProcessIo {
    fn read(&mut self, input: &str) -> Result<Vec<u8>, CliError> {
        if input == STDIO {
            if std::mem::replace(&mut self.stdin_read, true) {
                return Err(CliError::new(
                    ErrorCode::InvalidCommandLine,
                    "standard input can be read only once",
                ));
            }
            let stdin = std::io::stdin();
            // Nobody types a PDF: rather than wait, say what `-` expects (clig.dev).
            if stdin.is_terminal() {
                return Err(CliError::new(
                    ErrorCode::InvalidCommandLine,
                    "standard input is a terminal: pipe a PDF in, or name a file",
                ));
            }
            let mut bytes = Vec::new();
            return stdin
                .lock()
                .read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(|e| unreadable(input, &e));
        }
        fs::read(input).map_err(|e| unreadable(input, &e))
    }

    fn write(&mut self, output: &str, bytes: &[u8], force: bool) -> Result<(), CliError> {
        if output == STDIO {
            let mut stdout = std::io::stdout().lock();
            if stdout.is_terminal() {
                return Err(CliError::new(
                    ErrorCode::BinaryOutputToTerminal,
                    "standard output is a terminal",
                ));
            }
            return stdout
                .write_all(bytes)
                .and_then(|()| stdout.flush())
                .map_err(|e| unwritable(output, &e));
        }
        let path = Path::new(output);
        if !force && path.exists() {
            return Err(refused(output));
        }
        write_atomically(path, bytes).map_err(|e| unwritable(output, &e))
    }
}

/// Writes `bytes` to a temporary file beside `path`, then renames it over `path`.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let folder = match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder,
        _ => Path::new("."),
    };
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = folder.join(format!(".{name}.laminapdf-{}.tmp", std::process::id()));
    let written = fs::File::create(&temporary).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    match written.and_then(|()| fs::rename(&temporary, path)) {
        Ok(()) => Ok(()),
        Err(error) => {
            // The temporary file is ours; a failed removal leaves only it behind.
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// `input-unreadable` for `input`.
pub(crate) fn unreadable(input: &str, error: &std::io::Error) -> CliError {
    CliError::new(ErrorCode::InputUnreadable, "couldn't read an input")
        .with("input", input)
        .with("reason", format!("{:?}", error.kind()))
}

/// `output-unwritable` for `output`.
fn unwritable(output: &str, error: &std::io::Error) -> CliError {
    CliError::new(ErrorCode::OutputUnwritable, "couldn't write the output")
        .with("output", output)
        .with("reason", format!("{:?}", error.kind()))
}

/// `overwrite-refused` for `output`.
pub(crate) fn refused(output: &str) -> CliError {
    CliError::new(
        ErrorCode::OverwriteRefused,
        "refusing to replace a file without --force",
    )
    .with("output", output)
}

#[cfg(test)]
pub(crate) mod memory {
    //! An [`Io`] over in-memory files, for the command tests.

    use std::collections::BTreeMap;

    use super::{CliError, ErrorCode, Io, STDIO, refused};

    /// Files by name, standard input and output, and whether standard output is a terminal.
    #[derive(Debug, Default)]
    pub(crate) struct MemoryIo {
        pub(crate) files: BTreeMap<String, Vec<u8>>,
        pub(crate) stdin: Option<Vec<u8>>,
        pub(crate) stdout: Vec<u8>,
        pub(crate) stdout_is_terminal: bool,
    }

    impl Io for MemoryIo {
        fn read(&mut self, input: &str) -> Result<Vec<u8>, CliError> {
            let found = if input == STDIO {
                self.stdin.take()
            } else {
                self.files.get(input).cloned()
            };
            found.ok_or_else(|| super::unreadable(input, &std::io::ErrorKind::NotFound.into()))
        }

        fn write(&mut self, output: &str, bytes: &[u8], force: bool) -> Result<(), CliError> {
            if output == STDIO {
                if self.stdout_is_terminal {
                    return Err(CliError::new(
                        ErrorCode::BinaryOutputToTerminal,
                        "standard output is a terminal",
                    ));
                }
                self.stdout.extend_from_slice(bytes);
            } else if !force && self.files.contains_key(output) {
                return Err(refused(output));
            } else {
                self.files.insert(output.to_owned(), bytes.to_vec());
            }
            Ok(())
        }
    }
}
