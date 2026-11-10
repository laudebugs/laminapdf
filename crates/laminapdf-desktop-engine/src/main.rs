//! The `laminapdf-desktop-engine` binary: the main process starts it with piped stdio.
//!
//! `--pdfium <dir>` names the folder holding the PDFium library (the app's resources); without
//! it, `LAMINA_PDF_PDFIUM_DIR` is tried. Without PDFium, the engine still opens documents and
//! reports their pages, and answers every tile with `RenderUnavailable`.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use laminapdf_render::RenderBackend;
use laminapdf_render_pdfium::PdfiumBackend;

fn main() -> ExitCode {
    let backend = match pdfium_dir() {
        Ok(Some(dir)) => PdfiumBackend::bind(&dir),
        Ok(None) => PdfiumBackend::from_env(),
        Err(message) => {
            log(&message);
            return ExitCode::FAILURE;
        }
    };
    let backend: Option<Box<dyn RenderBackend>> = match backend {
        Ok(backend) => Some(Box::new(backend)),
        Err(error) => {
            log(&format!("rendering is off: {}", error.code()));
            None
        }
    };
    match laminapdf_desktop_engine::run(std::io::stdin().lock(), std::io::stdout(), backend) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log(&format!("stopped: {error}"));
            ExitCode::FAILURE
        }
    }
}

/// The folder after `--pdfium`, if given.
fn pdfium_dir() -> Result<Option<PathBuf>, String> {
    let mut args = std::env::args_os().skip(1);
    let mut dir = None;
    while let Some(arg) = args.next() {
        if arg == "--pdfium" {
            dir = Some(PathBuf::from(args.next().ok_or("--pdfium needs a folder")?));
        } else {
            return Err(format!("unknown argument {}", arg.to_string_lossy()));
        }
    }
    Ok(dir)
}

fn log(message: &str) {
    // Nothing to do if stderr is gone.
    let _ = writeln!(std::io::stderr().lock(), "{message}");
}
