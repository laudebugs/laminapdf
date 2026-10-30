//! Generates the hint table and the docs URL constant from `fixtures/codes.json` (ADR 0026
//! §5, §6) into `$OUT_DIR/codes.rs`, so `hint()` and `docs_url()` never drift from the file
//! and nothing parses JSON at run time. L1b replaces this with the generator for every surface.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a build script: failing the build loudly is the right behaviour"
)]

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let path = Path::new(&manifest).join("../../fixtures/codes.json");
    println!("cargo::rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let codes: serde_json::Value = serde_json::from_str(&text).expect("fixtures/codes.json is JSON");
    let url = codes.get("docsUrl").expect("codes.json has docsUrl");
    let field = |key: &str| {
        url.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("docsUrl.{key}"))
            .to_owned()
    };
    let mut out = String::new();
    writeln!(out, "/// The base of every `docsUrl` (ADR 0026 §6), from `codes.json`.").ok();
    writeln!(out, "pub(crate) const DOCS_BASE: &str = {:?};", field("base")).ok();
    writeln!(out, "/// The suffix of every `docsUrl`, from `codes.json`.").ok();
    writeln!(out, "pub(crate) const DOCS_SUFFIX: &str = {:?};", field("suffix")).ok();
    let groups = codes.as_object().expect("codes.json is an object");
    for field in [
        (
            "HINTS",
            "hint",
            "Every code's fixed hint (with its `{limit}`/`{option}` parameters)",
        ),
        ("SUMMARIES", "summary", "Every code's fixed one-sentence summary"),
    ] {
        let (name, key) = (field.0, field.1);
        let mut rows: Vec<(String, String)> = Vec::new();
        for (group_key, group) in groups {
            if matches!(group_key.as_str(), "version" | "docsUrl" | "limits") {
                continue;
            }
            for entry in group.as_array().into_iter().flatten() {
                if let (Some(code), Some(text)) = (
                    entry.get("code").and_then(serde_json::Value::as_str),
                    entry.get(key).and_then(serde_json::Value::as_str),
                ) {
                    rows.push((code.to_owned(), text.to_owned()));
                }
            }
        }
        rows.sort();
        writeln!(out, "/// {}, from `codes.json`, sorted by code.", field.2).ok();
        writeln!(out, "pub(crate) const {name}: &[(&str, &str)] = &[").ok();
        for (code, text) in rows {
            writeln!(out, "    ({code:?}, {text:?}),").ok();
        }
        writeln!(out, "];").ok();
    }
    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("codes.rs");
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("writing {}: {e}", dest.display()));
}
