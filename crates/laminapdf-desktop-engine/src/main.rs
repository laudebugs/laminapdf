//! The `laminapdf-desktop-engine` binary: the main process starts it with piped stdio, inside its
//! sandbox (`laminapdf-desktop-sandbox`).
//!
//! Arguments:
//!
//! - `--pdfium <dir>`: the folder holding the PDFium library (the app's resources); without it,
//!   `LAMINA_PDF_PDFIUM_DIR` is tried (for tests and the `measure` example). Without PDFium, the
//!   engine still opens documents and reports their pages, and answers every tile with
//!   `RenderUnavailable`.
//! - `--memory-limit <bytes>`: the sandbox's memory limit, which the main process passes.
//! - `--crash-socket <path>` and `--crash-server-pid <pid>`: the monitor's engine-dump server and
//!   the monitor's own process id (ADR 0014 §10). The engine connects and installs its crash
//!   handler first of all, before PDFium and the sandbox, so a native crash leaves a minidump; the
//!   sandbox then lets the handler use that one socket, and on Linux lets the monitor trace it while
//!   it writes the dump. Without them, or if the server can't be reached, the engine runs without
//!   capture.
//! - `--summarize-dump`: read a minidump's bytes from stdin, under the full sandbox, and write its
//!   allow-listed summary to stdout as JSON, then exit. The main process spawns the engine this way
//!   so the untrusted dump is parsed sandboxed and the parser never links into the main process.
//! - `--sandbox-selftest[=<probe>]` (feature `selftest`, which the tests and CI turn on): lock
//!   down, try what the sandbox must stop, print the results and exit (`selftest.rs`).
//!
//! The start-up order is the sandbox's (ADR 0008 §5): crash capture, then PDFium, then the engine
//! locks itself down, and only then reads its stdin, so no document is read before the sandbox is
//! on. `Ready` carries the sandbox's layers.

use std::path::PathBuf;
use std::process::ExitCode;

use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::sync::Arc;

use laminapdf_desktop_crash::summary::summarize_to_json;
use laminapdf_desktop_crash::{CRASH_SERVER_PID_ARG, CRASH_SOCKET_ARG, SUMMARIZE_DUMP_ARG};
use laminapdf_desktop_engine::fonts::{FontSource, Fonts};
use laminapdf_desktop_engine::{Setup, log};
use laminapdf_desktop_sandbox::{MEMORY_LIMIT_ARG, SandboxConfig, apply_engine_sandbox};
use laminapdf_render::{RenderBackend, RenderError};
use laminapdf_render_pdfium::{PDFIUM_DIR_ENV, PdfiumBackend};

/// The parsed command line.
#[derive(Default)]
struct Args {
    pdfium: Option<PathBuf>,
    crash_socket: Option<PathBuf>,
    crash_server: Option<u32>,
    config: SandboxConfig,
    mode: Mode,
}

/// What the engine was started for.
#[derive(Default)]
enum Mode {
    /// Serve the main process on stdin and stdout.
    #[default]
    Serve,
    /// Read a minidump from stdin under the sandbox and write its summary to stdout.
    SummarizeDump,
    /// Run the selftest: every probe (`None`), or one (feature `selftest`).
    #[cfg(feature = "selftest")]
    Selftest(Option<String>),
}

fn main() -> ExitCode {
    let mut args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            log(&message);
            return ExitCode::FAILURE;
        }
    };
    if matches!(args.mode, Mode::SummarizeDump) {
        return summarize_dump();
    }
    // Kept to the end: dropping it would detach the handler.
    let capture = connect_crash_server(args.crash_socket.as_deref(), args.crash_server);
    if capture.is_some() {
        // The socket the sandbox must leave usable: Linux's seccomp names it by descriptor.
        #[cfg(unix)]
        {
            args.config.crash_socket = capture.as_ref().and_then(Capture::descriptor);
        }
        args.config.crash_server = args.crash_server;
    }
    let fonts = Arc::new(Fonts::default());
    let backend: Option<Box<dyn RenderBackend>> = match bind(args.pdfium.as_deref(), &fonts) {
        Ok(backend) => Some(Box::new(backend)),
        Err(error) => {
            log(&format!("rendering is off: {}", error.code()));
            None
        }
    };
    let sandboxed = match apply_engine_sandbox(&args.config) {
        Ok(level) => level,
        Err(error) => {
            // Never serve untrusted documents outside the sandbox.
            log(&format!("the sandbox could not be applied: {}", report(&error)));
            return ExitCode::FAILURE;
        }
    };
    if sandboxed.is_off() {
        log("the sandbox is off (LAMINA_PDF_ENGINE_SANDBOX=off, a debug build)");
    }
    #[cfg(feature = "selftest")]
    if let Mode::Selftest(probe) = &args.mode {
        return match laminapdf_desktop_engine::selftest::run(probe.as_deref(), &sandboxed, &args.config, backend) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                log(&message);
                ExitCode::FAILURE
            }
        };
    }
    laminapdf_desktop_engine::crash::report_panics();
    let setup = Setup {
        backend,
        sandboxed,
        fonts,
    };
    match laminapdf_desktop_engine::run(std::io::stdin().lock(), std::io::stdout(), setup) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log(&format!("stopped: {error}"));
            ExitCode::FAILURE
        }
    }
}

/// Reads a minidump from stdin under the full sandbox and writes its summary to stdout as JSON. The
/// sandbox goes on before the untrusted dump is touched; the main process caps the time by ending
/// this process, so there is no timeout here.
fn summarize_dump() -> ExitCode {
    // No PDFium, no crash socket: this reads a dump, nothing else.
    if let Err(error) = apply_engine_sandbox(&SandboxConfig::default()) {
        log(&format!("the sandbox could not be applied: {}", report(&error)));
        return ExitCode::FAILURE;
    }
    let mut bytes = Vec::new();
    if std::io::stdin().lock().read_to_end(&mut bytes).is_err() {
        return ExitCode::FAILURE;
    }
    match summarize_to_json(bytes) {
        Ok(json) => {
            if std::io::stdout().lock().write_all(json.as_bytes()).is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            log(&format!("the dump could not be read: {error}"));
            ExitCode::FAILURE
        }
    }
}

use laminapdf_desktop_crash::CrashCapture as Capture;

/// Connects to the monitor's crash-dump server at `socket` and installs the crash handler; `server`
/// is the monitor's process id, the one process allowed to trace this one while it writes the dump
/// (Linux). Without both there is no capture at all, never a handler that lets any process trace it.
/// On Linux a capture whose socket descriptor couldn't be identified is dropped, closing the socket:
/// the lock-down would count it as a leaked descriptor and refuse to start, so the engine runs
/// without capture instead. On Windows the engine connects while its start-up impersonation still
/// holds the user's token, before the sandbox locks down; the connection stays usable after.
fn connect_crash_server(socket: Option<&Path>, server: Option<u32>) -> Option<Capture> {
    let (socket, server) = match (socket, server) {
        (Some(socket), Some(server)) => (socket, server),
        (None, None) => return None,
        _ => {
            log("native crashes won't leave a dump: --crash-socket and --crash-server-pid go together");
            return None;
        }
    };
    // The monitor may have just been spawned and not listen yet; a short wait, so an engine never
    // holds a window's first page for long.
    match laminapdf_desktop_crash::capture_crashes_within(socket, server, std::time::Duration::from_secs(2)) {
        #[cfg(target_os = "linux")]
        Ok(capture) if capture.descriptor().is_none() => {
            log("native crashes won't leave a dump: the crash socket's descriptor is unknown");
            drop(capture);
            None
        }
        Ok(capture) => Some(capture),
        Err(error) => {
            log(&format!("native crashes won't leave a dump: {}", report(&error)));
            None
        }
    }
}

/// Binds PDFium in `dir` (or `LAMINA_PDF_PDFIUM_DIR`) with its system fonts coming from the main
/// process. Windows keeps PDFium's own font mapper, which asks GDI and needs no file access.
fn bind(dir: Option<&Path>, fonts: &Arc<Fonts>) -> Result<PdfiumBackend, RenderError> {
    let env = std::env::var_os(PDFIUM_DIR_ENV).map(PathBuf::from);
    let Some(dir) = dir.map(Path::to_path_buf).or(env) else {
        return PdfiumBackend::from_env();
    };
    if cfg!(windows) {
        PdfiumBackend::bind(&dir)
    } else {
        PdfiumBackend::bind_with_fonts(&dir, Box::new(FontSource(Arc::clone(fonts))))
    }
}

/// An error with each of its causes.
fn report(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let _ = write!(text, ": {cause}");
        source = cause.source();
    }
    text
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args_os().skip(1);
    let mut parsed = Args::default();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if arg == "--pdfium" {
            parsed.pdfium = Some(PathBuf::from(args.next().ok_or("--pdfium needs a folder")?));
        } else if arg == CRASH_SOCKET_ARG {
            parsed.crash_socket = Some(PathBuf::from(args.next().ok_or("--crash-socket needs a path")?));
        } else if arg == CRASH_SERVER_PID_ARG {
            let value = args.next().ok_or("--crash-server-pid needs a number")?;
            parsed.crash_server = Some(
                value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "--crash-server-pid needs a number".to_owned())?,
            );
        } else if arg == SUMMARIZE_DUMP_ARG {
            parsed.mode = Mode::SummarizeDump;
        } else if arg == MEMORY_LIMIT_ARG {
            let value = args.next().ok_or("--memory-limit needs a number of bytes")?;
            parsed.config.memory_limit = value
                .to_string_lossy()
                .parse()
                .map_err(|_| "--memory-limit needs a number of bytes".to_owned())?;
        } else if text.starts_with("--sandbox-selftest") {
            parsed.mode = selftest_mode(&text)?;
        } else {
            return Err(format!("unknown argument {text}"));
        }
    }
    Ok(parsed)
}

#[cfg(feature = "selftest")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as without the feature, where it fails"
)]
fn selftest_mode(arg: &str) -> Result<Mode, String> {
    Ok(Mode::Selftest(
        arg.strip_prefix("--sandbox-selftest=").map(str::to_owned),
    ))
}

#[cfg(not(feature = "selftest"))]
fn selftest_mode(_: &str) -> Result<Mode, String> {
    Err("this engine was built without the selftest (feature `selftest`)".into())
}
