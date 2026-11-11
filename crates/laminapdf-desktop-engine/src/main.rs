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
//! - `--sandbox-selftest[=<probe>]` (feature `selftest`, which the tests and CI turn on): lock
//!   down, try what the sandbox must stop, print the results and exit (`selftest.rs`).
//!
//! The start-up order is the sandbox's (ADR 0008 §5): PDFium is loaded first, then the engine
//! locks itself down, and only then reads its stdin, so no document is read before the sandbox is
//! on. `Ready` carries the sandbox's layers.

use std::path::PathBuf;
use std::process::ExitCode;

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use laminapdf_desktop_engine::fonts::{FontSource, Fonts};
use laminapdf_desktop_engine::{Setup, log};
use laminapdf_desktop_sandbox::{MEMORY_LIMIT_ARG, SandboxConfig, apply_engine_sandbox};
use laminapdf_render::{RenderBackend, RenderError};
use laminapdf_render_pdfium::{PDFIUM_DIR_ENV, PdfiumBackend};

/// The parsed command line.
#[derive(Default)]
struct Args {
    pdfium: Option<PathBuf>,
    config: SandboxConfig,
    mode: Mode,
}

/// What the engine was started for.
#[derive(Default)]
enum Mode {
    /// Serve the main process on stdin and stdout.
    #[default]
    Serve,
    /// Run the selftest: every probe (`None`), or one (feature `selftest`).
    #[cfg(feature = "selftest")]
    Selftest(Option<String>),
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            log(&message);
            return ExitCode::FAILURE;
        }
    };
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
