//! npm licence check: the JavaScript half of the supply-chain rules (cargo-deny covers Rust).
//!
//! Production dependencies must use a licence from [`ALLOWED`], the same allow-list as
//! `deny.toml`. Dev-only tooling (bundlers, linters, test runners) never ships, so it is held to
//! a looser rule: no copyleft licence ([`COPYLEFT_PREFIXES`]). Licences are SPDX expressions;
//! `A OR B` passes when either side does, `A AND B` only when both do.

use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::workspace::Workspace;

/// Licences allowed for shipped dependencies. Keep in sync with `[licenses] allow` in `deny.toml`.
const ALLOWED: [&str; 10] = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "0BSD",
    "Unicode-3.0",
    "BSL-1.0",
];

/// SPDX identifier prefixes treated as copyleft (strong or network): rejected even for dev tooling.
const COPYLEFT_PREFIXES: [&str; 7] = ["GPL", "LGPL", "AGPL", "SSPL", "EUPL", "OSL", "CPAL"];

/// Checks the licences of every npm dependency in the pnpm workspace.
pub(crate) fn check_npm(ws: &Workspace) -> Result<()> {
    crate::step("npm licences (pnpm licenses list)");
    let mut problems = check_group(ws, "--prod", |id| ALLOWED.contains(&id))?;
    problems.extend(check_group(ws, "--dev", |id| {
        !COPYLEFT_PREFIXES.iter().any(|prefix| id.starts_with(prefix))
    })?);
    if problems.is_empty() {
        println!("all npm licences allowed");
        Ok(())
    } else {
        bail!(
            "npm dependencies with disallowed licences (see docs/guidelines/rust.md#dependencies):\n  {}",
            problems.join("\n  ")
        )
    }
}

/// Lists one dependency group (`--prod` or `--dev`) and returns the packages whose licence fails `allowed`.
fn check_group(ws: &Workspace, group: &str, allowed: impl Fn(&str) -> bool) -> Result<Vec<String>> {
    if !ws.root.join("node_modules").is_dir() {
        bail!("node_modules is missing: run `pnpm install` at the repository root first");
    }
    let output = Command::new("pnpm")
        .args(["licenses", "list", "--recursive", "--json", group])
        .current_dir(&ws.root)
        .output()
        .context("failed to run pnpm licenses list")?;
    if !output.status.success() {
        bail!("pnpm licenses list failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With no dependencies in the group pnpm prints a plain-text notice instead of JSON.
    if stdout.trim_start().starts_with("No licenses") {
        return Ok(Vec::new());
    }
    let Ok(Value::Object(by_licence)) = serde_json::from_str::<Value>(&stdout) else {
        bail!("unexpected output from pnpm licenses list {group}:\n{stdout}");
    };
    let mut problems = Vec::new();
    for (licence, packages) in &by_licence {
        if spdx_allows(licence, &allowed) {
            continue;
        }
        for package in packages.as_array().into_iter().flatten() {
            let name = package.get("name").and_then(Value::as_str).unwrap_or("?");
            problems.push(format!("{name} ({group}): {licence}"));
        }
    }
    Ok(problems)
}

/// Evaluates an SPDX licence expression: `OR` binds looser than `AND`, parentheses group, and
/// `X WITH exception` is one licence. Anything unparseable is rejected.
fn spdx_allows(expression: &str, allowed: &impl Fn(&str) -> bool) -> bool {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ");
    let mut tokens = spaced.split_whitespace().peekable();
    let result = or_expr(&mut tokens, allowed);
    result == Some(true) && tokens.next().is_none()
}

type Tokens<'a> = std::iter::Peekable<std::str::SplitWhitespace<'a>>;

fn or_expr(tokens: &mut Tokens<'_>, allowed: &impl Fn(&str) -> bool) -> Option<bool> {
    let mut value = and_expr(tokens, allowed)?;
    while tokens.next_if(|t| t.eq_ignore_ascii_case("OR")).is_some() {
        value |= and_expr(tokens, allowed)?;
    }
    Some(value)
}

fn and_expr(tokens: &mut Tokens<'_>, allowed: &impl Fn(&str) -> bool) -> Option<bool> {
    let mut value = term(tokens, allowed)?;
    while tokens.next_if(|t| t.eq_ignore_ascii_case("AND")).is_some() {
        value &= term(tokens, allowed)?;
    }
    Some(value)
}

fn term(tokens: &mut Tokens<'_>, allowed: &impl Fn(&str) -> bool) -> Option<bool> {
    let token = tokens.next()?;
    if token == "(" {
        let value = or_expr(tokens, allowed)?;
        return tokens.next_if_eq(&")").map(|_| value);
    }
    if tokens.next_if(|t| t.eq_ignore_ascii_case("WITH")).is_some() {
        let exception = tokens.next()?;
        return Some(allowed(&format!("{token} WITH {exception}")));
    }
    Some(allowed(token))
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED, spdx_allows};

    fn shipped(expression: &str) -> bool {
        spdx_allows(expression, &|id: &str| ALLOWED.contains(&id))
    }

    #[test]
    fn accepts_single_allowed_licence() {
        assert!(shipped("MIT"));
    }

    #[test]
    fn accepts_or_when_one_side_is_allowed() {
        assert!(shipped("(MIT OR GPL-3.0-only)"));
    }

    #[test]
    fn rejects_and_when_one_side_is_disallowed() {
        assert!(!shipped("MIT AND GPL-3.0-only"));
    }

    #[test]
    fn accepts_allowed_with_exception() {
        assert!(shipped("Apache-2.0 WITH LLVM-exception"));
    }

    #[test]
    fn rejects_unbalanced_parentheses() {
        assert!(!shipped("(MIT"));
    }

    #[test]
    fn rejects_unknown_licence_text() {
        assert!(!shipped("SEE LICENSE IN LICENSE.md"));
    }
}
