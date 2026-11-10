//! Third-party notices from cargo-about (pinned in doctor.rs), offline: the CLI's archives
//! (`cargo xtask cli-dist`) and the desktop app (`cargo xtask desktop-notices`). Each crate keeps
//! its own `about/about.toml` (the licences it may contain) and `about/notices.hbs`.

use std::path::Path;

use anyhow::{Context as _, Result};

use crate::workspace::Workspace;

/// Writes `out` for the crate in `dir` with `features` on, from its `about/` files.
pub(crate) fn generate(ws: &Workspace, dir: &Path, features: &[&str], out: &Path) -> Result<()> {
    crate::doctor::require(ws, &crate::doctor::cargo_about())?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let out_arg = out.to_string_lossy().into_owned();
    let joined = features.join(",");
    let mut args = vec![
        "about",
        "generate",
        "--locked",
        "--offline",
        "--fail",
        "--config",
        "about/about.toml",
    ];
    if !features.is_empty() {
        args.extend(["--features", joined.as_str()]);
    }
    args.extend(["about/notices.hbs", "--output-file", out_arg.as_str()]);
    crate::run(dir, "cargo", &args)
}

/// `cargo xtask desktop-notices`: the desktop app's `THIRD_PARTY_NOTICES.md` in target/desktop/,
/// with the MPL-2.0 text that Tauri's MPL-2.0 dependencies require (ADR 0008 §1, ADR 0019).
pub(crate) fn desktop(ws: &Workspace) -> Result<()> {
    let notices = ws.root.join("target/desktop/THIRD_PARTY_NOTICES.md");
    generate(ws, &ws.root.join("apps/desktop/src-tauri"), &[], &notices)?;
    println!("desktop-notices: third-party notices in {}", notices.display());
    Ok(())
}
