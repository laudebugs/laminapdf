//! `packages/render/src/generated/codes.ts`: the codes `@laminapdf/render`'s backends throw,
//! their kinds and the docs URL of ADR 0026 §6, from `fixtures/codes.json` (Q2l). The package
//! keeps no copy of these tables.

use std::fmt::Write as _;

use anyhow::{Result, bail};

use super::{Codes, quoted, ts_union};

/// The output.
pub(super) const TS: &str = "packages/render/src/generated/codes.ts";

/// The group whose codes are the render backends' own.
const GROUP: &str = "renderErrors";

/// Codes of other groups a render backend throws too, in the order the union lists them after
/// [`GROUP`]'s: laminapdf-render's `RenderError` reuses them, and the web adapters throw the same.
const REUSED: [&str; 8] = [
    "invalid-argument",
    "page-index-out-of-range",
    "password-required",
    "wrong-password",
    "data-pending",
    "source",
    "cancelled",
    "disposed",
];

/// The TS module: `RenderErrorCode`, `RenderErrorKind`, `KINDS` and `DOCS_URL`.
pub(super) fn render_ts(codes: &Codes) -> Result<String> {
    let mut rows: Vec<(&str, &str)> = codes
        .entries
        .iter()
        .filter(|e| e.group == GROUP)
        .map(|e| (e.code.as_str(), e.kind.as_str()))
        .collect();
    for code in REUSED {
        let Some(e) = codes.entries.iter().find(|e| e.code == code) else {
            bail!("{code}: a render backend throws it, but fixtures/codes.json has no such code");
        };
        rows.push((code, e.kind.as_str()));
    }
    let mut kinds: Vec<&str> = Vec::new();
    for (_, kind) in &rows {
        if !kinds.contains(kind) {
            kinds.push(kind);
        }
    }
    let mut out = String::new();
    let _ = writeln!(out, "{}", super::HEADER);
    let _ = writeln!(
        out,
        "// From fixtures/codes.json ({GROUP} and the codes it reuses); `cargo xtask check docs` fails when it is stale.\n"
    );
    let names: Vec<&str> = rows.iter().map(|(code, _)| *code).collect();
    ts_union(
        &mut out,
        "The codes a render backend throws (ADR 0026).",
        "RenderErrorCode",
        &names,
    );
    ts_union(
        &mut out,
        "What kind of failure a render code is (the ADR 0026 kinds its codes use).",
        "RenderErrorKind",
        &kinds,
    );
    let _ = writeln!(out, "/** The kind of each render code. */");
    let _ = writeln!(
        out,
        "export const KINDS: {{ readonly [C in RenderErrorCode]: RenderErrorKind }} = {{"
    );
    for (code, kind) in &rows {
        let _ = writeln!(out, "  {}: {},", quoted(code), quoted(kind));
    }
    out.push_str("};\n\n");
    let _ = writeln!(
        out,
        "/** Where each code's page is (ADR 0026 §6): `${{base}}${{code}}${{suffix}}`. */"
    );
    let _ = writeln!(
        out,
        "export const DOCS_URL = {{ base: {}, suffix: {} }} as const;",
        quoted(&codes.docs_base),
        quoted(&codes.docs_suffix)
    );
    Ok(out)
}
