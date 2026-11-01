//! The examples' lints (docs/guidelines/examples.md, "How it's checked"): Biome's examples override
//! in `check ts`, and `cargo xtask clippy-examples` in `check rust`. Both ratchet like `docs-prose`
//! ([`crate::reports`]): strict on the examples a PR changes, warnings on the others.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::reports::{Options, Ratchet, finding};
use crate::tracked::Tracked;
use crate::workspace::Workspace;

const TS_EXAMPLES: &str = "packages/sdk/examples/";
const RUST_EXAMPLES: &str = "crates/lamina/examples/";
/// The examples' clippy configuration: the root `clippy.toml` plus a 40-line function limit.
const CLIPPY_CONF_DIR: &str = "tools/xtask/clippy-examples";

/// Biome's `packages/sdk/biome.json` override warns on every example; this lints the changed
/// ones again with warnings as errors.
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
            path.starts_with(TS_EXAMPLES) && crate::util::has_extension(path, &["ts"]) && ws.root.join(path).is_file()
        })
        .filter(|path| ratchet.is_strict(path))
        .collect();
    crate::step(&format!(
        "examples (Biome): {} changed example(s) with warnings as errors",
        strict.len()
    ));
    if strict.is_empty() {
        return Ok(());
    }
    let mut args = vec!["exec", "biome", "lint", "--error-on-warnings"];
    args.extend(strict.iter().map(String::as_str));
    ws.pnpm(&args)
}

/// `cargo xtask clippy-examples [--all-strict] [--warnings] [--json]`: clippy on `lamina`'s
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
            "lamina",
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
        // would make each run re-check every crate under `lamina`.
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
        .filter(|path| path.starts_with(RUST_EXAMPLES) && crate::util::has_extension(path, &["rs"]))
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

/// A clippy warning in one of `lamina`'s examples, as `(path, finding)`.
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
    if !path.starts_with(RUST_EXAMPLES) {
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
                "spans": [{ "is_primary": true, "file_name": "crates/lamina/examples/invoice.rs", "line_start": 12 }],
            }
        });
        let found = example_finding(&message, Path::new("/repo"));
        assert!(
            found.is_some_and(|(path, finding)| path == "crates/lamina/examples/invoice.rs"
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
                "spans": [{ "is_primary": true, "file_name": "crates/lamina/src/lib.rs", "line_start": 1 }],
            }
        });
        assert!(example_finding(&message, Path::new("/repo")).is_none());
    }

    #[test]
    fn the_examples_clippy_config_should_start_with_the_root_one() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap_or_default();
        let examples = read(&format!("{CLIPPY_CONF_DIR}/clippy.toml"));
        assert!(examples.starts_with(&read("clippy.toml")));
        assert!(examples.contains("too-many-lines-threshold = 40"));
    }
}
