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
//! - `--sandbox-selftest[=<probe>]`: lock down, try what the sandbox must stop, print the results
//!   and exit (`selftest.rs`).
//!
//! The start-up order is the sandbox's (ADR 0008 §5): PDFium is loaded first, then the engine
//! locks itself down, and only then reads its stdin, so no document is read before the sandbox is
//! on. `Ready` carries the sandbox's layers.

use std::path::PathBuf;
use std::process::ExitCode;

use laminapdf_desktop_engine::{log, selftest};
use laminapdf_desktop_sandbox::{MEMORY_LIMIT_ARG, SandboxConfig, apply_engine_sandbox};
use laminapdf_render::RenderBackend;
use laminapdf_render_pdfium::PdfiumBackend;

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
    /// Run the selftest: every probe (`None`), or one.
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
    let backend = match &args.pdfium {
        Some(dir) => PdfiumBackend::bind(dir),
        None => PdfiumBackend::from_env(),
    };
    let backend: Option<Box<dyn RenderBackend>> = match backend {
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
            log(&format!("the sandbox could not be applied: {error}"));
            return ExitCode::FAILURE;
        }
    };
    if sandboxed.off {
        log("the sandbox is off (LAMINA_PDF_ENGINE_SANDBOX=off, a debug build)");
    }
    if let Mode::Selftest(probe) = &args.mode {
        return match selftest::run(probe.as_deref(), &sandboxed, &args.config, backend) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                log(&message);
                ExitCode::FAILURE
            }
        };
    }
    match laminapdf_desktop_engine::run(std::io::stdin().lock(), std::io::stdout(), backend, sandboxed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log(&format!("stopped: {error}"));
            ExitCode::FAILURE
        }
    }
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
        } else if text == "--sandbox-selftest" {
            parsed.mode = Mode::Selftest(None);
        } else if let Some(probe) = text.strip_prefix("--sandbox-selftest=") {
            parsed.mode = Mode::Selftest(Some(probe.to_owned()));
        } else {
            return Err(format!("unknown argument {text}"));
        }
    }
    Ok(parsed)
}
