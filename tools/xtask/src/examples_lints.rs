//! The examples' lints (docs/guidelines/examples.md, "How it's checked"): Biome's examples override
//! in `check ts`, and `cargo xtask clippy-examples` in `check rust`. Both ratchet like `docs-prose`
//! ([`crate::reports`]): run on their own, strict on the examples a PR changes and warnings on the
//! others; `cargo xtask check` runs both strict on every example.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::docs_snippets::{RUST_EXAMPLES, TS_EXAMPLE_DIRS, is_tool};
use crate::reports::{Options, Ratchet, finding};
use crate::tracked::Tracked;
use crate::workspace::Workspace;

/// The examples' clippy configuration: the root `clippy.toml` plus a 40-line function limit.
const CLIPPY_CONF_DIR: &str = "tools/xtask/clippy-examples";

/// Biome's examples overrides (`packages/sdk/biome.json`, `packages/viewer/biome.json`) report
/// errors on every example; this lints the examples again with warnings as errors (the changed ones, or every one with `--all-strict`), for
/// the rules that only warn outside the override.
pub(crate) fn biome(ws: &Workspace, options: Options) -> Result<()> {
    if options.json || options.warnings {
        bail!("usage: cargo xtask biome-examples [--all-strict] (Biome prints its own report)");
    }
    let untracked = Ratchet::untracked(&ws.root)?;
    let ratchet = Ratchet::new(&ws.root, "examples (Biome)", options, &untracked);
    let tracked = Tracked::from_git(&ws.root)?;
    let strict: Vec<String> = tracked
        .files()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .chain(untracked)
        .filter(|path| {
            TS_EXAMPLE_DIRS.iter().any(|dir| path.starts_with(dir))
                && crate::util::has_extension(path, &["ts"])
                && ws.root.join(path).is_file()
        })
        .filter(|path| ratchet.is_strict(path))
        .collect();
    crate::step(&if options.all_strict {
        format!(
            "examples (Biome): every example ({}) with warnings as errors",
            strict.len()
        )
    } else {
        format!(
            "examples (Biome): {} changed example(s) with warnings as errors",
            strict.len()
        )
    });
    if strict.is_empty() {
        return Ok(());
    }
    let mut args = vec!["exec", "biome", "lint", "--error-on-warnings"];
    args.extend(strict.iter().map(String::as_str));
    ws.pnpm(&args)
}

/// `cargo xtask clippy-examples [--all-strict] [--warnings] [--json]`: clippy on `laminapdf`'s
/// examples with [`CLIPPY_CONF_DIR`] and `let_underscore_must_use`; findings in a changed example
/// fail, the others are counted.
pub(crate) fn clippy(ws: &Workspace, options: Options) -> Result<()> {
    let untracked = Ratchet::untracked(&ws.root)?;
    let mut ratchet = Ratchet::new(&ws.root, "clippy-examples", options, &untracked);
    let output = Command::new(crate::program_path("cargo"))
        .args([
            "clippy",
            "--locked",
            "-p",
            "laminapdf",
            "--examples",
            "--all-features",
            "--message-format=json",
            "--",
            "-W",
            "clippy::too_many_lines",
            "-W",
            "clippy::let_underscore_must_use",
        ])
        .env("CLIPPY_CONF_DIR", ws.root.join(CLIPPY_CONF_DIR))
        // Its own build folder: these clippy arguments differ from `check rust`'s, so sharing one
        // would make each run re-check every crate under `laminapdf`.
        .env("CARGO_TARGET_DIR", target_dir(ws))
        .current_dir(&ws.root)
        .output()
        .context("running cargo clippy on lamina's examples")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for message in stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        if let Some((path, found)) = example_finding(&message, &ws.root) {
            ratchet.add(&path, [found]);
        }
    }
    let files = Tracked::from_git(&ws.root)?
        .files()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .chain(untracked)
        .filter(|path| path.starts_with(RUST_EXAMPLES) && !is_tool(path) && crate::util::has_extension(path, &["rs"]))
        .count();
    if !output.status.success() {
        bail!(
            "cargo clippy on lamina's examples failed (a build error, not a lint):\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    ratchet.finish(files, "docs/guidelines/examples.md")
}

/// `clippy-examples`' build folder: `clippy-examples/` in `$CARGO_TARGET_DIR` or `target/`.
fn target_dir(ws: &Workspace) -> std::path::PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| ws.root.join("target"), std::path::PathBuf::from)
        .join("clippy-examples")
}

/// A clippy warning in one of `laminapdf`'s examples, as `(path, finding)`.
fn example_finding(message: &Value, root: &Path) -> Option<(String, crate::reports::Finding)> {
    if message.get("reason")?.as_str()? != "compiler-message" {
        return None;
    }
    let diagnostic = message.get("message")?;
    if diagnostic.get("level")?.as_str()? != "warning" {
        return None;
    }
    let code = diagnostic.pointer("/code/code")?.as_str()?;
    let span = diagnostic
        .get("spans")?
        .as_array()?
        .iter()
        .find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))?;
    let file = span.get("file_name")?.as_str()?.replace('\\', "/");
    let root = root.to_string_lossy().replace('\\', "/");
    let path = file.strip_prefix(&format!("{root}/")).unwrap_or(&file).to_owned();
    if !path.starts_with(RUST_EXAMPLES) || is_tool(&path) {
        return None;
    }
    let line = span
        .get("line_start")?
        .as_u64()
        .and_then(|line| usize::try_from(line).ok())?;
    let text = diagnostic.get("message")?.as_str()?;
    let (rule, message) = match code {
        "clippy::too_many_lines" => (
            "function-length",
            format!("{text}: split it into functions of 40 lines or fewer"),
        ),
        "clippy::let_underscore_must_use" => (
            "discarded-result",
            "handle the `#[must_use]` result (with `?` or a `match`) instead of discarding it with `let _ =`"
                .to_owned(),
        ),
        _ => ("clippy", format!("{text} ({code})")),
    };
    Some((path, finding(line, rule, message)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_function_in_an_example_should_be_a_finding() {
        let message = serde_json::json!({
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "message": "this function has too many lines (55/40)",
                "code": { "code": "clippy::too_many_lines" },
                "spans": [{ "is_primary": true, "file_name": "crates/laminapdf/examples/invoice.rs", "line_start": 12 }],
            }
        });
        let found = example_finding(&message, Path::new("/repo"));
        assert!(
            found.is_some_and(|(path, finding)| path == "crates/laminapdf/examples/invoice.rs"
                && finding.line == 12
                && finding.rule == "function-length")
        );
    }

    #[test]
    fn warnings_outside_the_examples_should_be_ignored() {
        let message = serde_json::json!({
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "message": "x",
                "code": { "code": "clippy::too_many_lines" },
                "spans": [{ "is_primary": true, "file_name": "crates/laminapdf/src/lib.rs", "line_start": 1 }],
            }
        });
        assert!(example_finding(&message, Path::new("/repo")).is_none());
    }

    #[test]
    fn warnings_in_a_developer_tool_under_the_examples_should_be_ignored() {
        let message = serde_json::json!({
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "message": "this function has too many lines (43/40)",
                "code": { "code": "clippy::too_many_lines" },
                "spans": [{
                    "is_primary": true,
                    "file_name": "crates/laminapdf/examples/differential/text-facts.rs",
                    "line_start": 35,
                }],
            }
        });
        assert!(example_finding(&message, Path::new("/repo")).is_none());
    }

    /// The comment that opens the root `clippy.toml`'s last paragraph, its own threshold.
    const ROOT_THRESHOLD_MARKER: &str = "# Keep this paragraph last:";

    #[test]
    fn the_examples_clippy_config_should_start_with_the_root_one() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap_or_default();
        let examples = read(&format!("{CLIPPY_CONF_DIR}/clippy.toml"));
        // The root file ends with its own threshold, after this marker; the examples' file has
        // everything before it, then its own.
        let root_config = read("clippy.toml");
        let (shared, own) = root_config.split_once(ROOT_THRESHOLD_MARKER).unwrap();
        assert!(own.contains("too-many-lines-threshold = 100") && examples.starts_with(shared.trim_end()));
        assert!(examples.contains("too-many-lines-threshold = 40"));
    }
}
