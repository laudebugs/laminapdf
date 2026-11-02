//! `cargo xtask examples` (todo B5c): writes every example of the `laminapdf` façade crate into
//! `target/examples-out/` (through its test, which passes the fixed options), compares each output byte for byte with its golden in
//! `fixtures/golden/examples/` and runs `qpdf --check` on it (the full lane). `cargo xtask
//! golden --update` writes the goldens with the same runner ([`run_all`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::step;
use crate::workspace::Workspace;

/// The façade's examples: every `crates/laminapdf/examples/<name>.rs`, each writing `<name>.pdf`
/// or, for an example with variants (`pdf_a_archive`: PDF/A-2b and PDF/A-2u), one
/// `<name>-<variant>.pdf` per variant ([`outputs_of`]), in name order. Discovered, not listed, so a new example is a golden without an xtask edit
/// (`crates/laminapdf/tests/examples.rs` checks its own list against the same folder).
pub(crate) fn names(root: &Path) -> Result<Vec<String>> {
    let dir = root.join("crates/laminapdf/examples");
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

/// Writes every example's document into `out` (created if needed) and returns the files
/// written. The examples take their file ID, producer, dates and input files as parameters, so
/// the documents come from `crates/laminapdf/tests/examples.rs`, which passes the fixed values
/// (its ignored `write_every_example`), not from running each example's `main`.
pub(crate) fn run_all(ws: &Workspace, out: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let out_arg = out.to_string_lossy().into_owned();
    let args = [
        "test",
        "--locked",
        "-q",
        "-p",
        "laminapdf",
        "--test",
        "examples",
        "--",
        "--ignored",
        "--exact",
        "write_every_example",
    ];
    crate::run_env(&ws.root, "cargo", &args, &[("LAMINA_PDF_EXAMPLES_OUT", &out_arg)])?;
    let mut files = Vec::new();
    for name in names(&ws.root)? {
        let written = outputs_of(out, &name)?;
        if written.is_empty() {
            bail!(
                "example {name} wrote no {name}.pdf or {name}-<variant>.pdf in {}",
                out.display()
            );
        }
        files.extend(written);
    }
    Ok(files)
}

/// The files example `name` writes into `out`: `<name>.pdf` and every `<name>-<variant>.pdf`,
/// sorted.
pub(crate) fn outputs_of(out: &Path, name: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(out).with_context(|| format!("reading {}", out.display()))? {
        let path = entry?.path();
        let Some(file) = path.file_name().and_then(|f| f.to_str()) else {
            continue;
        };
        let Some(stem) = file.strip_suffix(".pdf") else {
            continue;
        };
        if stem == name || stem.strip_prefix(name).is_some_and(|rest| rest.starts_with('-')) {
            files.push(path);
        }
    }
    files.sort();
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

    #[test]
    fn an_example_with_variants_has_one_output_per_variant() {
        let dir = std::env::temp_dir().join(format!("lamina-xtask-outputs-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for f in ["a-2b.pdf", "a-2u.pdf", "ab.pdf", "a.txt", "b.pdf"] {
            fs::write(dir.join(f), b"").unwrap();
        }
        let found: Vec<String> = outputs_of(&dir, "a")
            .unwrap()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found, ["a-2b.pdf", "a-2u.pdf"]);
    }
}
