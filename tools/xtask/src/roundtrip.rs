//! `cargo xtask roundtrip [--tools pdfjs,pdfium,qpdf]` (ADR 0018 tier 7; todo B3c): lamina
//! saves every fixture × proving op × mode (laminapdf-save's `roundtrip` example, which also
//! reopens each output in lamina and checks the planner's decisions), then each output is
//! checked by `qpdf --check`, pdf.js (`tools/roundtrip/pdfjs.mjs`) and `PDFium`
//! (`tools/roundtrip/pdfium.py`, pypdfium2) against the manifest the example wrote.
//!
//! Needs qpdf on `PATH`, Node with pdf.js (`$LAMINA_PDF_PDFJS`, the path to a `pdfjs-dist` folder,
//! else `tools/roundtrip/node_modules/pdfjs-dist`, installed with npm from
//! `tools/roundtrip/package.json` when missing) and Python with pypdfium2
//! (`tools/roundtrip/requirements.txt`; `$LAMINA_PDF_PYTHON` or `python3`). The outputs, the
//! manifest and `versions.json` (every tool's version, for reproducing a failure) are in
//! `target/xtask/roundtrip/`.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::doctor::QPDF_VERSION;
use crate::workspace::Workspace;
use crate::{run, step};

/// The external tools, in the order they run.
pub(crate) const TOOLS: [&str; 3] = ["qpdf", "pdfjs", "pdfium"];

/// Parses `roundtrip [--tools a,b]`: the tools to run (default all of [`TOOLS`]).
pub(crate) fn parse_args(args: &[String]) -> Result<Vec<&'static str>> {
    match args {
        [] => Ok(TOOLS.to_vec()),
        [flag, list] if flag == "--tools" => list
            .split(',')
            .map(|t| {
                TOOLS
                    .into_iter()
                    .find(|known| *known == t.trim())
                    .with_context(|| format!("unknown tool {t:?}; tools: {}", TOOLS.join(", ")))
            })
            .collect(),
        _ => bail!("usage: cargo xtask roundtrip [--tools {}]", TOOLS.join(",")),
    }
}

/// Runs the suite with `tools`.
pub(crate) fn run_suite(ws: &Workspace, tools: &[&str]) -> Result<()> {
    let out = ws.root.join("target/xtask/roundtrip");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    let out_arg = out.to_string_lossy().into_owned();
    run(
        &ws.root,
        "cargo",
        &[
            "run",
            "--locked",
            "-q",
            "-p",
            "laminapdf-save",
            "--example",
            "roundtrip",
            "--",
            &out_arg,
        ],
    )?;
    let manifest: Value = serde_json::from_str(&fs::read_to_string(out.join("manifest.json"))?)?;
    let files: Vec<String> = manifest
        .get("cases")
        .and_then(Value::as_array)
        .context("manifest without cases")?
        .iter()
        .filter_map(|c| c.get("file").and_then(Value::as_str).map(str::to_owned))
        .collect();
    let mut versions = serde_json::Map::new();
    versions.insert("cases".to_owned(), json!(files.len()));
    let mut failed = Vec::new();
    for &tool in tools {
        // Each check reports the tool's version even when it fails, so a failure is
        // reproducible.
        let Checked { version, outcome } = match tool {
            "qpdf" => qpdf(&out, &files),
            "pdfjs" => pdfjs(ws, &out),
            _ => pdfium(ws, &out),
        };
        versions.insert(tool.to_owned(), version);
        if let Err(e) = outcome {
            eprintln!("{tool}: {e:#}");
            failed.push(tool);
        }
    }
    fs::write(out.join("versions.json"), serde_json::to_string_pretty(&versions)?)?;
    step(&format!("versions: {}", serde_json::to_string(&versions)?));
    if !failed.is_empty() {
        bail!("round trip failed in: {}", failed.join(", "));
    }
    step(&format!(
        "round trip: {} outputs clean in lamina and {}",
        files.len(),
        tools.join(", ")
    ));
    Ok(())
}

/// What one tool's check found: its version (`null` when it couldn't be read) and whether
/// every output passed.
struct Checked {
    version: Value,
    outcome: Result<()>,
}

impl Checked {
    fn failed(version: Value, e: anyhow::Error) -> Self {
        Self {
            version,
            outcome: Err(e),
        }
    }
}

/// `qpdf --check` on every output; exit status 0 only (3, warnings, is a failure too).
fn qpdf(out: &Path, files: &[String]) -> Checked {
    let version = match output("qpdf", &["--version"]) {
        Ok(text) => text.lines().next().unwrap_or_default().to_owned(),
        Err(e) => return Checked::failed(Value::Null, e),
    };
    let found = version.trim_start_matches("qpdf version ").trim();
    if found != QPDF_VERSION {
        return Checked::failed(
            json!(version),
            anyhow::anyhow!(
                "qpdf {found}, expected {QPDF_VERSION} (brew install qpdf, or a {QPDF_VERSION} release from https://github.com/qpdf/qpdf/releases)"
            ),
        );
    }
    step(&format!("qpdf --check on {} files", files.len()));
    let mut failures = Vec::new();
    for file in files {
        let result = match Command::new("qpdf").arg("--check").arg(out.join(file)).output() {
            Ok(result) => result,
            Err(e) => return Checked::failed(json!(version), anyhow::Error::new(e).context("running qpdf")),
        };
        if !result.status.success() {
            failures.push(format!(
                "{file}: {}\n{}{}",
                result.status,
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            ));
        }
    }
    let outcome = if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{} file(s) failed `qpdf --check`:\n{}",
            failures.len(),
            failures.join("\n")
        ))
    };
    Checked {
        version: json!(version),
        outcome,
    }
}

/// pdf.js over the manifest.
fn pdfjs(ws: &Workspace, out: &Path) -> Checked {
    let tools = ws.root.join("tools/roundtrip");
    let dir = if let Some(dir) = std::env::var_os("LAMINA_PDF_PDFJS") {
        dir.into()
    } else {
        let dir = tools.join("node_modules/pdfjs-dist");
        if !dir.join("package.json").is_file()
            && let Err(e) = run(
                &tools,
                "npm",
                &["install", "--no-package-lock", "--no-audit", "--no-fund"],
            )
        {
            return Checked::failed(Value::Null, e);
        }
        dir
    };
    let script = tools.join("pdfjs.mjs");
    tool_report("node", &[script.as_os_str(), dir.as_os_str(), out.as_os_str()])
}

/// `PDFium` (pypdfium2) over the manifest.
fn pdfium(ws: &Workspace, out: &Path) -> Checked {
    let script = ws.root.join("tools/roundtrip/pdfium.py");
    tool_report(&crate::harness::python(), &[script.as_os_str(), out.as_os_str()])
}

/// Runs a checker that prints one JSON line `{ tool, version, …, failures }`. The line without
/// its failures is the version; the check fails when it lists failures, exits unsuccessfully
/// or prints no report.
fn tool_report(program: &str, args: &[&std::ffi::OsStr]) -> Checked {
    let shown: Vec<_> = args.iter().map(|a| a.to_string_lossy()).collect();
    step(&format!("{program} {}", shown.join(" ")));
    let result = match Command::new(crate::program_path(program)).args(args).output() {
        Ok(result) => result,
        Err(e) => {
            return Checked::failed(
                Value::Null,
                anyhow::Error::new(e).context(format!("failed to start `{program}`; is it installed and on PATH?")),
            );
        }
    };
    let stdout = String::from_utf8_lossy(&result.stdout);
    let line = stdout.lines().last().unwrap_or_default();
    let Ok(mut report) = serde_json::from_str::<Value>(line) else {
        return Checked::failed(
            Value::Null,
            anyhow::anyhow!(
                "`{program}` printed no report ({}):\n{stdout}{}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            ),
        );
    };
    let failures = report
        .get("failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(map) = report.as_object_mut() {
        map.remove("failures");
    }
    let outcome = if failures.is_empty() && result.status.success() {
        Ok(())
    } else {
        let list: Vec<&str> = failures.iter().map(|f| f.as_str().unwrap_or_default()).collect();
        Err(anyhow::anyhow!(
            "{} failure(s) ({}):\n{}{}",
            list.len(),
            result.status,
            list.join("\n"),
            String::from_utf8_lossy(&result.stderr)
        ))
    };
    Checked {
        version: report,
        outcome,
    }
}

/// The stdout of `program args…`.
fn output(program: &str, args: &[&str]) -> Result<String> {
    let result = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to start `{program}`; is it installed and on PATH?"))?;
    if !result.status.success() {
        bail!("`{program} {}` failed with {}", args.join(" "), result.status);
    }
    Ok(String::from_utf8_lossy(&result.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn every_tool_runs_by_default() {
        assert_eq!(parse_args(&[]).unwrap(), TOOLS.to_vec());
    }

    #[test]
    fn tools_can_be_chosen() {
        assert_eq!(
            parse_args(&args(&["--tools", "qpdf,pdfium"])).unwrap(),
            ["qpdf", "pdfium"]
        );
    }

    #[test]
    fn an_unknown_tool_is_refused() {
        assert!(parse_args(&args(&["--tools", "mupdf"])).is_err());
    }
}
