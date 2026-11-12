//! Every public item of a published crate says which release added it: a line `Added in X.Y.Z.`
//! in its doc comment (docs/guidelines/rust.md, "Added in"). The check reads rustdoc's JSON
//! output, which is unstable, so `RUSTC_BOOTSTRAP=1` lets the pinned stable toolchain write it,
//! as cargo-semver-checks does. It walks each crate from its root, following re-exports, and
//! checks:
//!
//! - the free items: structs, enums, unions, traits, functions, constants, statics, type aliases
//!   and macros;
//! - the associated items of their inherent impls, and the items a public trait declares.
//!
//! Struct fields and enum variants are part of their type and need no line of their own. Items
//! hidden with `#[doc(hidden)]` are left out, as the docs leave them out. Crates are documented
//! with `--all-features` on the host, so an item behind a target `cfg` is checked only on that
//! target.

use crate::step;
use crate::workspace::Workspace;
use anyhow::{Context as _, Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The item kinds that need a line, as rustdoc's JSON names them.
const KINDS: &[&str] = &[
    "struct",
    "enum",
    "union",
    "trait",
    "trait_alias",
    "function",
    "constant",
    "static",
    "type_alias",
    "macro",
];

/// One public item without an `Added in` line: where it is and what it is called.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Missing {
    pub(crate) file: String,
    pub(crate) line: u64,
    pub(crate) name: String,
}

/// Whether `docs` has an `Added in X.Y.Z.` line.
pub(crate) fn has_added_in(docs: &str) -> bool {
    docs.lines().any(|line| {
        line.trim()
            .strip_prefix("Added in ")
            .and_then(|rest| rest.strip_suffix('.'))
            .is_some_and(is_version)
    })
}

/// `X.Y.Z`, three dot-separated numbers.
fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The items of one crate's rustdoc JSON that need a line and lack it.
pub(crate) fn missing(doc: &Value) -> Vec<Missing> {
    let Some(index) = doc.get("index").and_then(Value::as_object) else {
        return Vec::new();
    };
    let item = |id: &Value| index.get(&id_key(id));
    let mut seen = BTreeSet::new();
    let mut checked = BTreeSet::new();
    let mut stack: Vec<Value> = doc.get("root").cloned().into_iter().collect();
    while let Some(id) = stack.pop() {
        let key = id_key(&id);
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(it) = item(&id) else { continue };
        if it.get("crate_id").and_then(Value::as_u64) != Some(0) || hidden(it) {
            continue;
        }
        let Some((kind, inner)) = it.get("inner").and_then(Value::as_object).and_then(|o| o.iter().next()) else {
            continue;
        };
        match kind.as_str() {
            "module" => stack.extend(array(inner, "items")),
            "use" => stack.extend(inner.get("id").filter(|v| !v.is_null()).cloned()),
            _ => {
                if KINDS.contains(&kind.as_str()) && public(it) {
                    checked.insert(key);
                }
                if kind == "trait" && public(it) {
                    checked.extend(array(inner, "items").iter().map(id_key));
                }
                if matches!(kind.as_str(), "struct" | "enum" | "union") {
                    for imp in array(inner, "impls").iter().filter_map(item) {
                        let Some(imp) = imp.pointer("/inner/impl") else {
                            continue;
                        };
                        let inherent = imp.get("trait").is_none_or(Value::is_null)
                            && !imp.get("is_synthetic").and_then(Value::as_bool).unwrap_or(false)
                            && imp.get("blanket_impl").is_none_or(Value::is_null);
                        if inherent {
                            checked.extend(
                                array(imp, "items")
                                    .iter()
                                    .filter(|m| item(m).is_some_and(|m| public(m) && !hidden(m)))
                                    .map(id_key),
                            );
                        }
                    }
                }
            }
        }
    }
    let mut out: Vec<Missing> = checked
        .iter()
        .filter_map(|key| index.get(key))
        .filter(|it| !has_added_in(it.get("docs").and_then(Value::as_str).unwrap_or_default()))
        .map(|it| Missing {
            file: it
                .pointer("/span/filename")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_owned(),
            line: it.pointer("/span/begin/0").and_then(Value::as_u64).unwrap_or(0),
            name: it.get("name").and_then(Value::as_str).unwrap_or("?").to_owned(),
        })
        .collect();
    out.sort();
    out
}

/// An item ID as the index's key: rustdoc writes IDs as numbers and keys as their strings.
fn id_key(id: &Value) -> String {
    id.as_str().map_or_else(|| id.to_string(), str::to_owned)
}

fn array(value: &Value, key: &str) -> Vec<Value> {
    value.get(key).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn public(item: &Value) -> bool {
    item.get("visibility").and_then(Value::as_str) == Some("public")
}

/// `#[doc(hidden)]`, which rustdoc's JSON keeps among the item's attributes.
fn hidden(item: &Value) -> bool {
    array(item, "attrs")
        .iter()
        .any(|a| a.to_string().contains("doc(hidden)"))
}

/// `cargo xtask check since`: rustdoc JSON for every published crate, then every public item
/// checked for its `Added in` line.
pub(crate) fn check(ws: &Workspace) -> Result<()> {
    step("since: every public item of a published crate has an `Added in X.Y.Z.` line");
    let mut report = String::new();
    let mut count = 0;
    for name in ws.publishable_crates() {
        crate::run_env(
            &ws.root,
            "cargo",
            &[
                "rustdoc",
                "--locked",
                "--quiet",
                "-p",
                name,
                "--all-features",
                "--lib",
                "--",
                "-Z",
                "unstable-options",
                "--output-format",
                "json",
            ],
            &[("RUSTC_BOOTSTRAP", "1")],
        )?;
        let path = ws
            .target_dir
            .join("doc")
            .join(format!("{}.json", name.replace('-', "_")));
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let doc: Value = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        for m in missing(&doc) {
            count += 1;
            let _ = writeln!(report, "  {}:{}: {}", m.file, m.line, m.name);
        }
    }
    if count > 0 {
        bail!(
            "{count} public item(s) without an `Added in X.Y.Z.` line in their docs (add one naming the next release; docs/guidelines/rust.md):\n{report}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_added_in_line_needs_a_full_version_and_a_full_stop() {
        assert!(has_added_in("Opens a file.\n\nAdded in 0.2.0."));
        assert!(has_added_in("Added in 10.0.12."));
        assert!(!has_added_in("Added in 0.2."));
        assert!(!has_added_in("Added in 0.2.0"));
        assert!(!has_added_in("It was added in 0.2.0."));
        assert!(!has_added_in(""));
    }

    /// A root module with a documented struct, an undocumented function, a re-export, a hidden
    /// item, a trait with a method, and an inherent impl with a public and a private method.
    fn crate_json() -> Value {
        let item = |name: &str, docs: &str, inner: Value| {
            json!({ "crate_id": 0, "name": name, "docs": docs, "visibility": "public", "attrs": [],
                    "span": { "filename": "src/lib.rs", "begin": [1, 0] }, "inner": inner })
        };
        let mut private = item("helper", "", json!({ "function": {} }));
        private["visibility"] = json!("crate");
        let mut hidden = item("internal", "", json!({ "function": {} }));
        hidden["attrs"] = json!(["#[doc(hidden)]"]);
        json!({
            "root": 0,
            "index": {
                "0": item("lib", "", json!({ "module": { "items": [1, 2, 3, 5, 7] } })),
                "1": item("Doc", "A thing.\n\nAdded in 0.2.0.", json!({ "struct": { "impls": [8] } })),
                "2": item("open", "Opens.", json!({ "function": {} })),
                "3": item("reexport", "", json!({ "use": { "id": 4 } })),
                "4": item("Again", "", json!({ "enum": { "impls": [] } })),
                "5": hidden,
                "7": item("Visit", "Added in 0.2.0.", json!({ "trait": { "items": [6] } })),
                "6": item("visit", "Visits.", json!({ "function": {} })),
                "8": json!({ "crate_id": 0, "visibility": "default", "attrs": [],
                             "inner": { "impl": { "trait": null, "is_synthetic": false,
                                                  "blanket_impl": null, "items": [9, 10] } } }),
                "9": item("new", "Makes one.", json!({ "function": {} })),
                "10": private,
            }
        })
    }

    #[test]
    fn finds_every_public_item_without_a_line_and_nothing_else() {
        let names: Vec<String> = missing(&crate_json()).into_iter().map(|m| m.name).collect();
        assert_eq!(names, ["Again", "new", "open", "visit"]);
    }
}
