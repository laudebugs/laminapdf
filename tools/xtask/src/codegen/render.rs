//! `packages/render/src/generated/codes.ts`: the codes `@laminapdf/render`'s backends throw,
//! their kinds and the docs URL of ADR 0026 §6, from `fixtures/codes.json` and the codes
//! laminapdf-render's `RenderError::error_code` returns (Q2l). The package keeps no copy of these
//! tables.

use std::fmt::Write as _;

use anyhow::{Context, Result, bail};

use super::{Codes, quoted, ts_union, variant};

/// The output.
pub(super) const TS: &str = "packages/render/src/generated/codes.ts";

/// The group whose codes are the render backends' own.
const GROUP: &str = "renderErrors";

/// The Rust source whose `RenderError::error_code` names every code laminapdf-render throws.
pub(super) const RUST_ERRORS: &str = "crates/laminapdf-render/src/error.rs";

/// Codes only the web adapters throw (a closed `RenderDocument`; in Rust, dropping closes).
const WEB_ONLY: [&str; 1] = ["disposed"];

/// The `ErrorCode::X` variants `RenderError::error_code` returns, in order, each once.
fn rust_codes(source: &str) -> Result<Vec<&str>> {
    let start = source
        .find("pub const fn error_code(&self) -> ErrorCode {")
        .with_context(|| format!("{RUST_ERRORS}: no RenderError::error_code"))?;
    let mut variants = Vec::new();
    for line in source[start..].lines().skip(1).take_while(|l| l.trim() != "}") {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for (at, _) in line.match_indices("ErrorCode::") {
            let rest = &line[at + "ErrorCode::".len()..];
            let variant = rest
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or_default();
            if !variant.is_empty() && !variants.contains(&variant) {
                variants.push(variant);
            }
        }
    }
    if variants.is_empty() {
        bail!("{RUST_ERRORS}: RenderError::error_code names no ErrorCode");
    }
    Ok(variants)
}

/// The TS module: `RenderErrorCode`, `RenderErrorKind`, `KINDS` and `DOCS_URL`. The codes are
/// [`GROUP`]'s, then the other codes `rust_errors` (laminapdf-render's `error.rs`) maps a
/// variant to, then [`WEB_ONLY`].
pub(super) fn render_ts(codes: &Codes, rust_errors: &str) -> Result<String> {
    let mut rows: Vec<(&str, &str)> = codes
        .entries
        .iter()
        .filter(|e| e.group == GROUP)
        .map(|e| (e.code.as_str(), e.kind.as_str()))
        .collect();
    let reused = rust_codes(rust_errors)?
        .into_iter()
        .map(|v| {
            codes
                .entries
                .iter()
                .find(|e| variant(&e.code) == v)
                .with_context(|| format!("{RUST_ERRORS}: ErrorCode::{v} is no code of fixtures/codes.json"))
        })
        .collect::<Result<Vec<_>>>()?;
    for e in reused {
        if !rows.iter().any(|(c, _)| *c == e.code) {
            rows.push((e.code.as_str(), e.kind.as_str()));
        }
    }
    for code in WEB_ONLY {
        let Some(e) = codes.entries.iter().find(|e| e.code == code) else {
            bail!("{code}: the web adapters throw it, but fixtures/codes.json has no such code");
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
