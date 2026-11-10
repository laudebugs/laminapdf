//! `cargo xtask fixtures large-pdf [--pages N] [--out PATH]`: the desktop engine's 1 GB fixture,
//! written by the engine crate's `large_pdf` example with the `laminapdf` builder, so xtask itself
//! doesn't link the core. Never committed: `target/fixtures/` is ignored.

use anyhow::Result;

use crate::desktop_sidecar::ENGINE;
use crate::workspace::Workspace;

/// Runs the `large_pdf` example with `args`.
pub(crate) fn large_pdf(ws: &Workspace, args: &[String]) -> Result<()> {
    let mut cargo = vec![
        "run",
        "--locked",
        "--release",
        "-q",
        "-p",
        ENGINE,
        "--example",
        "large_pdf",
        "--",
    ];
    cargo.extend(args.iter().map(String::as_str));
    ws.cargo(&cargo)
}
