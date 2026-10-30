//! `cargo xtask gen codes [--check]`: `@lamina/pdf`'s code unions from `fixtures/codes.json`
//! (ADR 0026 §3, typescript.md §errors: codes shared with Rust are generated, never written by
//! hand) into `packages/pdf/src/codes.generated.ts`. `--check` fails when the file is stale.
//!
//! The file holds types (no runtime cost), the code lists the tests compare with, and the kind
//! and hint of the codes the TS layer raises itself (every other code arrives from the WASM
//! with its kind and hint already filled in). The output is formatted the way Biome formats it
//! (one line when it fits in 120 columns, else one member per line) and then passed through
//! `biome format` itself, so the generated file and `biome ci` always agree.

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::workspace::Workspace;

#[path = "../../../crates/lamina-cos/tests/common/code_groups.rs"]
mod code_groups;

use code_groups::{NOT_CODE_GROUPS, TS_ONLY_GROUPS, is_warning_group};

/// Where the generated module goes.
const OUT: &str = "packages/pdf/src/codes.generated.ts";

/// Codes of other groups that `@lamina/pdf` raises itself; each must be listed in `codes.json`.
const ALSO_RAISED_BY_TS: [&str; 4] = [
    "builder-finished",
    "invalid-argument",
    "nested-transaction",
    "no-transaction",
];

/// Biome's line width (`biome.json`).
const LINE_WIDTH: usize = 120;

fn out_path(ws: &Workspace) -> PathBuf {
    ws.root.join(OUT)
}

/// `cargo xtask gen codes [--check]`.
pub(crate) fn codes(ws: &Workspace, check: bool) -> Result<()> {
    let path = ws.root.join("fixtures/codes.json");
    let json: Value =
        serde_json::from_str(&fs::read_to_string(&path)?).with_context(|| format!("{} is not JSON", path.display()))?;
    let generated = biome_format(ws, &render(&json)?)?;
    let out = out_path(ws);
    let current = fs::read_to_string(&out).unwrap_or_default();
    if check {
        if current != generated {
            bail!("{OUT} is stale: run `cargo xtask gen codes`");
        }
        crate::step("gen codes --check: packages/pdf/src/codes.generated.ts is up to date");
        return Ok(());
    }
    if current != generated {
        fs::write(&out, generated)?;
    }
    println!("wrote {}", out.display());
    Ok(())
}

/// `source` as `biome format` writes it (Biome reads the root `biome.json`).
fn biome_format(ws: &Workspace, source: &str) -> Result<String> {
    ws.require_node_modules()?;
    let mut child = Command::new(crate::program_path("pnpm"))
        .args(["exec", "biome", "format", "--stdin-file-path", OUT])
        .current_dir(&ws.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running biome format through pnpm")?;
    child
        .stdin
        .take()
        .context("biome's stdin")?
        .write_all(source.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!("biome format failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    String::from_utf8(output.stdout).context("biome's output is not UTF-8")
}

/// One code entry.
struct Entry<'a> {
    group: &'a str,
    code: &'a str,
    kind: &'a str,
    hint: &'a str,
}

/// Every code of every group, groups in alphabetical order of their key (a stable order that
/// doesn't depend on how the JSON map is stored), codes in file order within a group.
fn entries(json: &Value) -> Result<Vec<Entry<'_>>> {
    let groups = json.as_object().context("codes.json is not an object")?;
    let mut keys: Vec<&String> = groups
        .keys()
        .filter(|k| !NOT_CODE_GROUPS.contains(&k.as_str()))
        .collect();
    keys.sort();
    let mut out = Vec::new();
    for group in keys {
        let list = groups.get(group).and_then(Value::as_array);
        for entry in list.with_context(|| format!("codes.json: {group} is not a list"))? {
            let field = |k: &str| {
                entry
                    .get(k)
                    .and_then(Value::as_str)
                    .with_context(|| format!("codes.json: an entry of {group} has no {k}"))
            };
            out.push(Entry {
                group,
                code: field("code")?,
                kind: field("kind")?,
                hint: entry.get("hint").and_then(Value::as_str).unwrap_or_default(),
            });
        }
    }
    Ok(out)
}

/// A JSON string literal, which is also a valid TS one.
fn quoted(s: &str) -> String {
    Value::String(s.to_owned()).to_string()
}

/// `export type name = …;`: `never` when empty, one line when it fits, else one member per line.
fn union(out: &mut String, doc: &str, name: &str, members: &[&str]) {
    let _ = writeln!(out, "/** {doc} */");
    let head = format!("export type {name} =");
    let one_line = format!(
        "{head} {};",
        if members.is_empty() {
            "never".to_owned()
        } else {
            members.iter().map(|m| quoted(m)).collect::<Vec<_>>().join(" | ")
        }
    );
    if one_line.len() <= LINE_WIDTH {
        let _ = writeln!(out, "{one_line}\n");
        return;
    }
    let _ = writeln!(out, "{head}");
    for (i, m) in members.iter().enumerate() {
        let end = if i + 1 == members.len() { ";" } else { "" };
        let _ = writeln!(out, "  | {}{end}", quoted(m));
    }
    out.push('\n');
}

/// `export const name: readonly ty[] = […];`: one line when it fits, else one member per line.
fn list(out: &mut String, doc: &str, name: &str, ty: &str, members: &[&str]) {
    let _ = writeln!(out, "/** {doc} */");
    let head = format!("export const {name}: readonly {ty}[] = [");
    let one_line = format!(
        "{head}{}];",
        members.iter().map(|m| quoted(m)).collect::<Vec<_>>().join(", ")
    );
    if one_line.len() <= LINE_WIDTH {
        let _ = writeln!(out, "{one_line}\n");
        return;
    }
    let _ = writeln!(out, "{head}");
    for m in members {
        let _ = writeln!(out, "  {},", quoted(m));
    }
    out.push_str("];\n\n");
}

/// The TS module for `json`.
fn render(json: &Value) -> Result<String> {
    let all = entries(json)?;
    if let Some(missing) = ALSO_RAISED_BY_TS.iter().find(|c| !all.iter().any(|e| e.code == **c)) {
        bail!("codes.json does not list {missing:?}, which @lamina/pdf raises (ALSO_RAISED_BY_TS)");
    }
    let errors: Vec<&str> = all
        .iter()
        .filter(|e| !is_warning_group(e.group))
        .map(|e| e.code)
        .collect();
    let warnings: Vec<&str> = all
        .iter()
        .filter(|e| is_warning_group(e.group))
        .map(|e| e.code)
        .collect();
    let limits: Vec<&str> = json
        .get("limits")
        .and_then(Value::as_array)
        .context("codes.json has no limits")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let docs = |k: &str| json.pointer(&format!("/docsUrl/{k}")).and_then(Value::as_str);
    let (Some(base), Some(suffix)) = (docs("base"), docs("suffix")) else {
        bail!("codes.json has no docsUrl.base / docsUrl.suffix");
    };

    let mut out = String::from(
        "// Generated by `cargo xtask gen codes` from fixtures/codes.json: do not edit.\n\
         // `cargo xtask check ts` fails when it is stale.\n\n",
    );
    union(
        &mut out,
        "Every error code (ADR 0026 §3; `fixtures/codes.json`).",
        "ErrorCode",
        &errors,
    );
    union(
        &mut out,
        "Every warning code (ADR 0026 §3; `fixtures/codes.json`).",
        "WarningCode",
        &warnings,
    );
    union(
        &mut out,
        "The names of the `Limits` (ADR 0003 §3), as `data.limit` of `limit-exceeded`.",
        "LimitName",
        &limits,
    );
    let order = "groups in alphabetical order of their key, codes in file order";
    list(
        &mut out,
        &format!("Every error code: {order}."),
        "ERROR_CODES",
        "ErrorCode",
        &errors,
    );
    list(
        &mut out,
        &format!("Every warning code: {order}."),
        "WARNING_CODES",
        "WarningCode",
        &warnings,
    );
    let _ = writeln!(
        out,
        "/** `docsUrl` = base + code + suffix (ADR 0026 §6). */\nexport const DOCS_URL = {{ base: {}, suffix: {} }} as const;\n",
        quoted(base),
        quoted(suffix)
    );
    out.push_str(
        "/** Kind and hint of the codes `@lamina/pdf` raises itself (the WASM fills in the others). */\n\
         export const RAISED_BY_PACKAGE = {\n",
    );
    for e in all
        .iter()
        .filter(|e| TS_ONLY_GROUPS.contains(&e.group) || ALSO_RAISED_BY_TS.contains(&e.code))
    {
        let _ = writeln!(
            out,
            "  {}: {{\n    kind: {},\n    hint: {},\n  }},",
            quoted(e.code),
            quoted(e.kind),
            quoted(e.hint)
        );
    }
    out.push_str("} as const;\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::render;

    /// A `codes.json` with the codes `ALSO_RAISED_BY_TS` needs, plus `extra` groups.
    fn json(extra: &serde_json::Value) -> serde_json::Value {
        let mut base = serde_json::json!({
            "version": 2,
            "docsUrl": { "base": "b/", "suffix": ".md" },
            "limits": ["max_x"],
            "bindingErrors": [
                { "code": "builder-finished", "kind": "state", "hint": "h" },
                { "code": "invalid-argument", "kind": "usage", "hint": "h" },
            ],
            "opErrors": [
                { "code": "nested-transaction", "kind": "state", "hint": "h" },
                { "code": "no-transaction", "kind": "state", "hint": "h" },
            ],
        });
        if let (Some(map), Some(more)) = (base.as_object_mut(), extra.as_object()) {
            map.extend(more.clone());
        }
        base
    }

    #[test]
    fn render_should_split_errors_from_warnings_by_group_name() {
        let ts = render(&json(&serde_json::json!({
            "warnings": [{ "code": "w", "kind": "input" }],
            "fontWarnings": [{ "code": "fw", "kind": "input" }],
        })))
        .unwrap_or_default();
        assert!(ts.contains("export type WarningCode = \"fw\" | \"w\";"), "{ts}");
    }

    #[test]
    fn render_should_write_a_short_union_on_one_line() {
        let ts = render(&json(&serde_json::json!({ "warnings": [] }))).unwrap_or_default();
        assert!(ts.contains("export type LimitName = \"max_x\";"), "{ts}");
    }

    #[test]
    fn render_should_write_never_and_an_empty_list_for_an_empty_group() {
        let ts = render(&json(&serde_json::json!({ "warnings": [] }))).unwrap_or_default();
        assert!(
            ts.contains("export type WarningCode = never;")
                && ts.contains("WARNING_CODES: readonly WarningCode[] = [];"),
            "{ts}"
        );
    }

    #[test]
    fn render_should_break_a_long_union_one_member_per_line() {
        let codes: Vec<serde_json::Value> = (0..20)
            .map(|i| serde_json::json!({ "code": format!("error-number-{i}"), "kind": "input" }))
            .collect();
        let ts = render(&json(&serde_json::json!({ "errors": codes, "warnings": [] }))).unwrap_or_default();
        assert!(
            ts.contains("export type ErrorCode =\n  | \"builder-finished\"\n"),
            "{ts}"
        );
    }

    #[test]
    fn render_should_list_package_codes_with_their_hint() {
        let ts = render(&json(&serde_json::json!({
            "warnings": [],
            "packageErrors": [{ "code": "p", "kind": "state", "hint": "do it" }],
        })))
        .unwrap_or_default();
        assert!(
            ts.contains("  \"p\": {\n    kind: \"state\",\n    hint: \"do it\",\n  },"),
            "{ts}"
        );
    }

    #[test]
    fn render_should_fail_when_a_code_raised_by_ts_is_not_listed() {
        let mut codes = json(&serde_json::json!({ "warnings": [] }));
        if let Some(map) = codes.as_object_mut() {
            map.remove("opErrors");
        }
        assert!(render(&codes).is_err());
    }
}
