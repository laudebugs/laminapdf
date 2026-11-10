//! `cargo xtask cli-docs`: the `laminapdf` CLI's man pages (one per command, `clap_mangen`) and
//! shell completions (`clap_complete`) into `target/cli-docs/`, for packaging. Never committed:
//! both come from the clap definitions, so they can't go stale. `cargo xtask cli-dist` adds the
//! third-party notices: together, the files dist-workspace.toml's `include` puts in every
//! release archive (todo L4d).

use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::workspace::Workspace;

/// The shells `laminapdf completions` writes a script for, and each script's file name.
const SHELLS: [(&str, &str); 5] = [
    ("bash", "laminapdf.bash"),
    ("zsh", "_laminapdf"),
    ("fish", "laminapdf.fish"),
    ("powershell", "_laminapdf.ps1"),
    ("elvish", "laminapdf.elv"),
];

pub(crate) fn run(ws: &Workspace) -> Result<()> {
    let out = ws.root.join("target/cli-docs");
    let man = out.join("man");
    let completions = out.join("completions");
    for dir in [&man, &completions] {
        if dir.exists() {
            std::fs::remove_dir_all(dir).with_context(|| format!("emptying {}", dir.display()))?;
        }
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    ws.cargo(&["build", "--locked", "-q", "-p", "laminapdf-cli"])?;
    let binary = ws
        .target_dir
        .join("debug")
        .join(format!("laminapdf{}", std::env::consts::EXE_SUFFIX));
    let man_arg = man.to_string_lossy().into_owned();
    crate::run(&ws.root, &binary.to_string_lossy(), &["man-pages", man_arg.as_str()])?;
    for (shell, file) in SHELLS {
        let output = Command::new(&binary)
            .args(["completions", shell])
            .output()
            .with_context(|| format!("running {}", binary.display()))?;
        if !output.status.success() {
            bail!(
                "`laminapdf completions {shell}` failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::fs::write(completions.join(file), output.stdout)?;
    }
    let pages = std::fs::read_dir(&man)?.count();
    println!(
        "cli-docs: {pages} man pages in {}, {} completion scripts in {}",
        man.display(),
        SHELLS.len(),
        completions.display()
    );
    Ok(())
}

/// `cargo xtask cli-dist`: [`run`], then `THIRD_PARTY_NOTICES.md` for the release binary's
/// dependencies (the `mcp` feature on, as dist builds it) from cargo-about, offline.
pub(crate) fn dist_files(ws: &Workspace) -> Result<()> {
    run(ws)?;
    let notices = ws.root.join("target/cli-docs/THIRD_PARTY_NOTICES.md");
    crate::notices::generate(ws, &ws.root.join("crates/laminapdf-cli"), &["mcp"], &notices)?;
    println!("cli-dist: third-party notices in {}", notices.display());
    Ok(())
}
