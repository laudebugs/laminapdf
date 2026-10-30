//! `cargo xtask examples` (todo B5c): runs every example of the `lamina` façade crate into
//! `target/examples-out/`, compares each output byte for byte with its golden in
//! `fixtures/golden/examples/` and runs `qpdf --check` on it (the full lane). `cargo xtask
//! golden --update` writes the goldens with the same runner ([`run_all`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::step;
use crate::workspace::Workspace;

/// The façade's examples: every `crates/lamina/examples/<name>.rs`, each writing `<name>.pdf`,
/// in name order. Discovered, not listed, so a new example is a golden without an xtask edit
/// (`crates/lamina/tests/examples.rs` checks its own list against the same folder).
pub(crate) fn names(root: &Path) -> Result<Vec<String>> {
    let dir = root.join("crates/lamina/examples");
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "rs")
            && let Some(stem) = path.file_stem()
        {
            names.push(stem.to_string_lossy().into_owned());
        }
    }
    names.sort();
    if names.is_empty() {
        bail!("no examples in {}", dir.display());
    }
    Ok(names)
}

/// Where `cargo xtask examples` writes the outputs.
pub(crate) const OUT_DIR: &str = "target/examples-out";

/// Runs every example into `out` (created if needed) and returns the files written.
pub(crate) fn run_all(ws: &Workspace, out: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let out_arg = out.to_string_lossy().into_owned();
    // One build for all of them, then each example on its own.
    ws.cargo(&["build", "--locked", "-q", "-p", "lamina", "--examples"])?;
    let mut files = Vec::new();
    for name in names(&ws.root)? {
        let name = name.as_str();
        ws.cargo(&[
            "run",
            "--locked",
            "-q",
            "-p",
            "lamina",
            "--example",
            name,
            "--",
            &out_arg,
        ])?;
        let file = out.join(format!("{name}.pdf"));
        if !file.is_file() {
            bail!("example {name} wrote no {}", file.display());
        }
        files.push(file);
    }
    Ok(files)
}

/// `cargo xtask examples`: run, compare with the goldens, `qpdf --check`.
pub(crate) fn run(ws: &Workspace) -> Result<()> {
    crate::doctor::require(ws, &crate::doctor::qpdf())?;
    let out = ws.root.join(OUT_DIR);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    let files = run_all(ws, &out)?;
    let golden = ws.root.join("fixtures/golden/examples");
    let mut problems = Vec::new();
    for file in &files {
        let name = file.file_name().context("file name")?;
        let committed = fs::read(golden.join(name)).unwrap_or_default();
        if fs::read(file)? != committed {
            problems.push(format!(
                "{}: differs from fixtures/golden/examples/ (run `cargo xtask golden --update` and review the diff)",
                name.to_string_lossy()
            ));
        }
        let check = Command::new("qpdf")
            .arg("--check")
            .arg(file)
            .output()
            .context("running qpdf")?;
        // Exit status 0 only: 3 (warnings) is a failure too.
        if !check.status.success() {
            problems.push(format!(
                "{}: qpdf --check {}\n{}{}",
                name.to_string_lossy(),
                check.status,
                String::from_utf8_lossy(&check.stdout),
                String::from_utf8_lossy(&check.stderr)
            ));
        }
    }
    if !problems.is_empty() {
        bail!("examples failed:\n  {}", problems.join("\n  "));
    }
    step(&format!(
        "examples: {} outputs match their goldens and pass qpdf --check ({OUT_DIR})",
        files.len()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_examples_are_found() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let names = names(&root).unwrap();
        assert!(names.iter().any(|n| n == "invoice"), "{names:?}");
    }
}
