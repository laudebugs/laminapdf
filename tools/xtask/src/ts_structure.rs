//! The maintainability guideline's structure checks for TypeScript (todo Q2u), run by
//! `cargo xtask check ts`: dependency-cruiser (no import cycles, no relative import into another
//! package or app), jscpd (no duplicated blocks in `packages/*/src`) and knip (no unused files,
//! exports or dependencies), through the root `package.json` scripts.
//!
//! dependency-cruiser's known violations (`.dependency-cruiser-known-violations.json`) only
//! shrink: before the usual run, this checks that every listed violation still occurs, so an
//! entry another todo fixed must be removed rather than left to excuse the same import later.

use std::collections::BTreeSet;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::workspace::Workspace;

const KNOWN: &str = ".dependency-cruiser-known-violations.json";

/// A violation as dependency-cruiser names it: rule, importing module, imported module.
type Violation = (String, String, String);

pub(crate) fn check(ws: &Workspace) -> Result<()> {
    crate::step("TypeScript structure: import cycles, duplicates, unused code");
    let known =
        read_violations(&std::fs::read_to_string(ws.root.join(KNOWN)).with_context(|| format!("reading {KNOWN}"))?)?;
    let found = current_violations(ws)?;
    let stale = stale(&known, &found);
    if !stale.is_empty() {
        let list: Vec<String> = stale
            .iter()
            .map(|(rule, from, to)| format!("  {rule}: {from} → {to}"))
            .collect();
        bail!(
            "{KNOWN} lists violations that no longer occur; remove them (the list only shrinks):\n{}",
            list.join("\n")
        );
    }
    for script in ["check:deps", "check:duplicates", "check:unused"] {
        ws.pnpm(&["run", script])?;
    }
    Ok(())
}

/// Every violation dependency-cruiser finds without the known list.
fn current_violations(ws: &Workspace) -> Result<BTreeSet<Violation>> {
    let output = Command::new(crate::program_path("pnpm"))
        .args([
            "exec",
            "depcruise",
            "--config",
            ".dependency-cruiser.json",
            "--no-ignore-known",
            "--output-type",
            "json",
            "packages",
            "apps",
        ])
        .current_dir(&ws.root)
        .output()
        .context("running dependency-cruiser through pnpm")?;
    let json: Value = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "reading dependency-cruiser's JSON (exit {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })?;
    let violations = json
        .pointer("/summary/violations")
        .context("dependency-cruiser's JSON has no summary.violations")?;
    read_violations(&violations.to_string())
}

/// The (rule, from, to) of each violation in a JSON array (the known list or a summary).
fn read_violations(json: &str) -> Result<BTreeSet<Violation>> {
    let value: Value = serde_json::from_str(json)?;
    let entries = value.as_array().context("expected a JSON array of violations")?;
    entries
        .iter()
        .map(|v| {
            let text = |p: &str| v.pointer(p).and_then(Value::as_str).map(str::to_owned);
            Ok((
                text("/rule/name").context("a violation without rule.name")?,
                text("/from").context("a violation without from")?,
                text("/to").context("a violation without to")?,
            ))
        })
        .collect()
}

/// The known violations that no longer occur.
fn stale(known: &BTreeSet<Violation>, found: &BTreeSet<Violation>) -> Vec<Violation> {
    known.difference(found).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(rule: &str, from: &str, to: &str) -> Violation {
        (rule.to_owned(), from.to_owned(), to.to_owned())
    }

    #[test]
    fn read_violations_should_take_rule_from_and_to() {
        let json =
            r#"[{"type":"dependency","from":"a.ts","to":"b.ts","rule":{"severity":"error","name":"no-circular"}}]"#;
        assert_eq!(
            read_violations(json).unwrap(),
            BTreeSet::from([v("no-circular", "a.ts", "b.ts")])
        );
    }

    #[test]
    fn stale_should_list_known_violations_that_no_longer_occur() {
        let known = BTreeSet::from([v("r", "a", "b"), v("r", "c", "d")]);
        let found = BTreeSet::from([v("r", "a", "b"), v("r", "e", "f")]);
        assert_eq!(stale(&known, &found), vec![v("r", "c", "d")]);
    }

    #[test]
    fn stale_should_be_empty_when_every_known_violation_occurs() {
        let known = BTreeSet::from([v("r", "a", "b")]);
        assert_eq!(stale(&known, &known.clone()), Vec::<Violation>::new());
    }
}
