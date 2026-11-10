//! The capability guard (ADR 0008 §6): every later todo keeps it passing.
//!
//! It reads `tauri.conf.json`, `tauri.e2e.conf.json` (when it exists) and `capabilities/*.json`,
//! and fails when:
//! - a capability grants a forbidden permission (`shell:`, `fs:`, `http:`, `dialog:`, `process:`):
//!   the webview never gets those, only Rust uses the plugins;
//! - a capability file other than `e2e.json` is missing from `app.security.capabilities` (with an
//!   explicit list, a file nobody listed is silently off), or the list names an identifier with no
//!   file;
//! - `e2e` is in the base list (only `tauri.e2e.conf.json` may enable it, D2e);
//! - `tauri.e2e.conf.json` doesn't list exactly the base capabilities plus `e2e`: `--config` merges
//!   as a JSON merge patch, which replaces arrays rather than appending to them.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

const FORBIDDEN: [&str; 5] = ["shell:", "fs:", "http:", "dialog:", "process:"];

/// A capability file's identifier and the permission identifiers it grants.
struct Capability {
    file: String,
    identifier: String,
    permissions: Vec<String>,
}

fn capability(file: &str, json: &Value) -> Capability {
    let permissions = json
        .get("permissions")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|p| p.as_str().or_else(|| p.get("identifier").and_then(Value::as_str)))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Capability {
        file: file.to_owned(),
        identifier: json
            .get("identifier")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        permissions,
    }
}

fn listed(conf: &Value) -> Vec<String> {
    conf.pointer("/app/security/capabilities")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Every problem the guard finds; empty when the configuration is sound.
fn problems(conf: &Value, caps: &[Capability], e2e_conf: Option<&Value>) -> Vec<String> {
    let mut found = Vec::new();
    for cap in caps {
        for permission in &cap.permissions {
            if FORBIDDEN.iter().any(|prefix| permission.starts_with(prefix)) {
                found.push(format!("{} grants the forbidden permission {permission}", cap.file));
            }
        }
    }
    let base = listed(conf);
    if base.is_empty() {
        found.push("tauri.conf.json has no app.security.capabilities list: Tauri would enable every file".into());
    }
    for cap in caps.iter().filter(|c| c.identifier != "e2e") {
        if !base.contains(&cap.identifier) {
            found.push(format!(
                "{} ({}) is not in app.security.capabilities",
                cap.file, cap.identifier
            ));
        }
    }
    for id in &base {
        if id == "e2e" {
            found.push("e2e is in tauri.conf.json's capabilities: only tauri.e2e.conf.json may enable it".into());
        } else if !caps.iter().any(|c| &c.identifier == id) {
            found.push(format!(
                "app.security.capabilities names {id}, which no capability file defines"
            ));
        }
    }
    if let Some(e2e) = e2e_conf {
        let mut expected = base;
        expected.push("e2e".into());
        if listed(e2e) != expected {
            found.push(format!(
                "tauri.e2e.conf.json must list {expected:?} (a merge patch replaces the base list), not {:?}",
                listed(e2e)
            ));
        }
    }
    found
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?)
}

#[test]
fn the_app_configuration_should_pass_the_capability_guard() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let conf = read_json(&root.join("tauri.conf.json"))?;
    let e2e_path = root.join("tauri.e2e.conf.json");
    let e2e = if e2e_path.exists() {
        Some(read_json(&e2e_path)?)
    } else {
        None
    };
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(root.join("capabilities"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|x| x == "json") {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            files.insert(name, read_json(&path)?);
        }
    }
    let caps: Vec<Capability> = files.iter().map(|(file, json)| capability(file, json)).collect();
    assert!(!caps.is_empty(), "no capability files");
    let found = problems(&conf, &caps, e2e.as_ref());
    assert_eq!(found, Vec::<String>::new(), "{found:#?}");
    Ok(())
}

fn conf(list: &[&str]) -> Value {
    serde_json::json!({ "app": { "security": { "capabilities": list } } })
}

fn cap(file: &str, identifier: &str, permissions: &[&str]) -> Capability {
    capability(
        file,
        &serde_json::json!({ "identifier": identifier, "permissions": permissions }),
    )
}

#[test]
fn the_guard_should_reject_a_forbidden_permission() {
    let caps = [cap("main.json", "main", &["allow-app-info", "fs:default"])];
    assert_eq!(problems(&conf(&["main"]), &caps, None).len(), 1);
}

#[test]
fn the_guard_should_reject_a_forbidden_permission_written_as_an_object() {
    let caps = [capability(
        "main.json",
        &serde_json::json!({ "identifier": "main", "permissions": [{ "identifier": "shell:allow-execute", "allow": [] }] }),
    )];
    assert_eq!(problems(&conf(&["main"]), &caps, None).len(), 1);
}

#[test]
fn the_guard_should_reject_an_unlisted_capability_file() {
    let caps = [cap("main.json", "main", &[]), cap("settings.json", "settings", &[])];
    assert_eq!(problems(&conf(&["main"]), &caps, None).len(), 1);
}

#[test]
fn the_guard_should_reject_a_listed_capability_without_a_file() {
    let caps = [cap("main.json", "main", &[])];
    assert_eq!(problems(&conf(&["main", "settings"]), &caps, None).len(), 1);
}

#[test]
fn the_guard_should_reject_e2e_in_the_base_list() {
    let caps = [cap("main.json", "main", &[]), cap("e2e.json", "e2e", &[])];
    assert_eq!(problems(&conf(&["main", "e2e"]), &caps, None).len(), 1);
}

#[test]
fn the_guard_should_reject_an_e2e_override_that_drops_a_base_entry() {
    let caps = [
        cap("main.json", "main", &[]),
        cap("settings.json", "settings", &[]),
        cap("e2e.json", "e2e", &[]),
    ];
    let e2e = conf(&["main", "e2e"]);
    assert_eq!(problems(&conf(&["main", "settings"]), &caps, Some(&e2e)).len(), 1);
}

#[test]
fn the_guard_should_accept_an_e2e_override_that_repeats_the_base_list() {
    let caps = [
        cap("main.json", "main", &[]),
        cap("e2e.json", "e2e", &["allow-app-info"]),
    ];
    let e2e = conf(&["main", "e2e"]);
    assert_eq!(problems(&conf(&["main"]), &caps, Some(&e2e)), Vec::<String>::new());
}

#[test]
fn the_guard_should_reject_a_missing_capability_list() {
    let caps = [cap("main.json", "main", &[])];
    assert_eq!(problems(&serde_json::json!({}), &caps, None).len(), 2);
}
