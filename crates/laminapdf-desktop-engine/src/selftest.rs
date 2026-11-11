//! `--sandbox-selftest[=<probe>[:<target>]]`: after locking down exactly as it does to serve, the
//! engine tries what the sandbox must stop and what it must still allow, and prints one line per
//! probe on stdout: `selftest <probe>: blocked (…)` or `allowed (…)`. The first line names the
//! sandbox's layers. The tests in `tests/sandbox_selftest.rs` read these lines (ADR 0008 §5).
//!
//! | Probe | Tries | Target |
//! | --- | --- | --- |
//! | `read` | reading a file | a path; default `/etc/hosts` or `C:\Windows\win.ini` |
//! | `write` | creating a file | a path; default a file in the user's home |
//! | `connect` | a TCP connection | an address; default `127.0.0.1:9` |
//! | `spawn` | starting a process | — (`/bin/sh` or `cmd.exe`) |
//! | `threads` | 4 threads that each open a page and render it (a font lookup too), then join | — |
//! | `memory` | allocating twice the memory limit, then a little more | — |
//! | `fork`, `exec` | `fork()`, `execve("/bin/sh")` (Unix) | — |
//! | `clone3` | a raw `clone3` (Linux): the sandbox answers `ENOSYS` | — |
//! | `nproc` | prints `RLIMIT_NPROC` (Linux), which must be the parent's | — |
//!
//! Without a probe, every probe runs in that order. On Linux a forbidden system call kills the
//! process (SIGSYS) instead of failing, so each forbidden probe needs its own run there.

use std::io::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use laminapdf_desktop_sandbox::probe::{self, Outcome};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxLevel};
use laminapdf_render::{ByteSource, CancelToken, RenderBackend, RenderOpenOptions, RenderRequest};

/// Every probe, in the order a run without a probe name tries them.
const PROBES: &[&str] = &[
    "read", "write", "connect", "spawn", "threads", "memory", "fork", "exec", "clone3", "nproc",
];

/// Runs `probe` (or every probe) and prints the results.
///
/// # Errors
/// An unknown probe's name.
pub fn run(
    probe: Option<&str>,
    level: &SandboxLevel,
    config: &SandboxConfig,
    backend: Option<Box<dyn RenderBackend>>,
) -> Result<(), String> {
    say("sandbox", &level.describe());
    let backend = backend.map(|backend| Arc::new(Mutex::new(backend)));
    let probes: Vec<&str> = match probe {
        Some(probe) => vec![probe],
        None => PROBES.to_vec(),
    };
    for probe in probes {
        let (name, target) = probe
            .split_once(':')
            .map_or((probe, None), |(name, target)| (name, Some(target)));
        let outcome = match name {
            "read" => probe::read(&target.map_or_else(default_read, PathBuf::from)),
            "write" => probe::write(&target.map_or_else(default_write, PathBuf::from)),
            "connect" => {
                let addr = target.unwrap_or("127.0.0.1:9");
                match addr.parse::<SocketAddr>() {
                    Ok(addr) => probe::connect(addr),
                    Err(_) => return Err(format!("not an address: {addr}")),
                }
            }
            "spawn" => spawn(),
            "threads" => threads(backend.as_ref()),
            "memory" => probe::allocate(config.memory_limit.saturating_mul(2) + (256 << 20)),
            #[cfg(unix)]
            "fork" => probe::fork(),
            #[cfg(unix)]
            "exec" => probe::exec(Path::new("/bin/sh")),
            #[cfg(target_os = "linux")]
            "clone3" => probe::clone3(),
            #[cfg(target_os = "linux")]
            "nproc" => match probe::nproc_limit() {
                Some((soft, hard)) => Outcome::Allowed(format!("{soft} {hard}")),
                None => Outcome::Blocked("getrlimit failed".into()),
            },
            // Probes of another OS's mechanisms: nothing to try here.
            #[cfg(not(unix))]
            "fork" | "exec" => continue,
            #[cfg(not(target_os = "linux"))]
            "clone3" | "nproc" => continue,
            other => return Err(format!("unknown probe {other}")),
        };
        say(name, &outcome.to_string());
    }
    Ok(())
}

fn say(name: &str, what: &str) {
    let mut out = std::io::stdout().lock();
    // A selftest whose stdout is gone has no one to tell.
    let _ = writeln!(out, "selftest {name}: {what}");
    let _ = out.flush();
}

fn default_read() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\Windows\win.ini")
    } else {
        PathBuf::from("/etc/hosts")
    }
}

fn default_write() -> PathBuf {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    let dir = home.map_or_else(std::env::temp_dir, PathBuf::from);
    dir.join(format!("laminapdf-sandbox-probe-{}", std::process::id()))
}

fn spawn() -> Outcome {
    if cfg!(windows) {
        probe::spawn(Path::new(r"C:\Windows\System32\cmd.exe"), &["/c", "exit 0"])
    } else {
        probe::spawn(Path::new("/bin/sh"), &["-c", "exit 0"])
    }
}

/// Four threads that each open the probe page with Lamina PDF and, with PDFium, render a tile of
/// it, then join: threads and rendering must survive the sandbox.
fn threads(backend: Option<&Arc<Mutex<Box<dyn RenderBackend>>>>) -> Outcome {
    let pdf: Arc<[u8]> = probe_pdf().into();
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (pdf, backend) = (Arc::clone(&pdf), backend.cloned());
            thread::Builder::new().spawn(move || render_once(&pdf, backend.as_ref()))
        })
        .collect();
    let mut rendered = 0;
    for worker in workers {
        match worker
            .map_err(|e| e.to_string())
            .and_then(|w| w.join().map_err(|_| "panicked".to_owned()))
        {
            Ok(Ok(tile)) => rendered += usize::from(tile),
            Ok(Err(why)) | Err(why) => return Outcome::Blocked(why),
        }
    }
    Outcome::Allowed(format!("4 threads joined, {rendered} tiles rendered"))
}

/// Opens the page; renders a 64 px tile when there is a renderer. `Ok(true)` when it rendered.
fn render_once(pdf: &Arc<[u8]>, backend: Option<&Arc<Mutex<Box<dyn RenderBackend>>>>) -> Result<bool, String> {
    let doc = laminapdf::Document::open_bytes(pdf.to_vec(), laminapdf::OpenOptions::new())
        .map_err(|e| e.code().to_owned())?;
    if doc.page_count().map_err(|e| e.code().to_owned())? != 1 {
        return Err("the probe page did not open".into());
    }
    let Some(backend) = backend else {
        return Ok(false);
    };
    let source: Arc<dyn ByteSource> = Arc::new(Arc::clone(pdf));
    let opened = backend
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open(source, &RenderOpenOptions::new())
        .map_err(|e| e.code().to_owned())?;
    let request = RenderRequest::page(0, 1.0);
    let plan = opened.plan(&request).map_err(|e| e.code().to_owned())?;
    let mut pixels = vec![0; plan.byte_len()];
    opened
        .render(&request, &mut pixels, &CancelToken::new())
        .map_err(|e| e.code().to_owned())?;
    // The page is a black square on white: some pixel must be dark.
    if pixels.as_chunks::<4>().0.iter().any(|[r, ..]| *r < 128) {
        Ok(true)
    } else {
        Err("the tile came out blank".into())
    }
}

/// A one-page PDF, 64 × 64 pt: a black square and a word in Helvetica, which the file doesn't
/// embed, so PDFium looks for a system font (and, sandboxed, falls back to its built-in one).
fn probe_pdf() -> Vec<u8> {
    let content = "0 g 16 16 32 32 re f BT /F1 12 Tf 4 4 Td (Probe) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 64 64] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}
