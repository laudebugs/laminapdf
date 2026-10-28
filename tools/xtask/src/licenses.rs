//! npm licence check: the JavaScript half of the supply-chain rules (cargo-deny covers Rust).
//!
//! Production dependencies must use a licence from `[licenses] allow` in `deny.toml`: one
//! allow-list for both ecosystems, so they can't drift (ADR 0019). Dev-only tooling (bundlers,
//! linters, test runners) is not shipped, so licences outside the list are reported but not
//! blocked. Licences are SPDX expressions; `A OR B` passes when either side does, `A AND B` only
//! when both do.

use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::workspace::Workspace;

/// Checks the licences of every npm dependency in the pnpm workspace.
pub(crate) fn check_npm(ws: &Workspace) -> Result<()> {
    crate::step("npm licences (pnpm licenses list, allow-list from deny.toml)");
    ws.require_node_modules()?;
    let deny_toml = fs::read_to_string(ws.root.join("deny.toml")).context("reading deny.toml")?;
    let allow_list = allow_list(&deny_toml)?;
    let allowed = |id: &str| allow_list.iter().any(|a| a == id);

    for entry in disallowed(ws, "--dev", &allowed)? {
        println!("note: dev-only dependency outside the allow-list (not shipped, not blocked): {entry}");
    }
    let problems = disallowed(ws, "--prod", &allowed)?;
    if problems.is_empty() {
        println!("all production npm licences allowed");
        Ok(())
    } else {
        bail!(
            "production npm dependencies with licences outside deny.toml's allow-list (ADR 0019):\n  {}",
            problems.join("\n  ")
        )
    }
}

/// `[licenses] allow` from `deny.toml`.
fn allow_list(deny_toml: &str) -> Result<Vec<String>> {
    let table: toml::Table = toml::from_str(deny_toml).context("deny.toml is not valid TOML")?;
    let list: Vec<String> = table
        .get("licenses")
        .and_then(|licenses| licenses.get("allow"))
        .and_then(toml::Value::as_array)
        .context("deny.toml has no [licenses] allow list")?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .context("[licenses] allow entries must be strings")
        })
        .collect::<Result<_>>()?;
    if list.is_empty() {
        bail!("deny.toml's [licenses] allow list is empty");
    }
    Ok(list)
}

/// Lists one dependency group (`--prod` or `--dev`) and returns `name (group): licence` for each
/// package whose licence expression `allowed` rejects.
fn disallowed(ws: &Workspace, group: &str, allowed: &impl Fn(&str) -> bool) -> Result<Vec<String>> {
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
    let mut found = Vec::new();
    for (licence, packages) in &by_licence {
        if spdx_allows(licence, allowed) {
            continue;
        }
        for package in packages.as_array().into_iter().flatten() {
            let name = package.get("name").and_then(Value::as_str).unwrap_or("?");
            found.push(format!("{name} ({group}): {licence}"));
        }
    }
    Ok(found)
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
    use super::{allow_list, spdx_allows};

    // `[bans] allow` (a crate list) comes first on purpose: only `[licenses] allow` may be read.
    const DENY_TOML: &str = r#"
[bans]
allow = [{ name = "foo" }]

[licenses]
allow = [
    "MIT",
    "Apache-2.0 WITH LLVM-exception", # comment
]
"#;

    fn shipped(expression: &str) -> bool {
        let list = allow_list(DENY_TOML).unwrap();
        spdx_allows(expression, &|id: &str| list.iter().any(|a| a == id))
    }

    #[test]
    fn allow_list_fails_without_licenses_table() {
        assert!(allow_list("[bans]\nallow = [\"MIT\"]\n").is_err());
    }

    #[test]
    fn reads_allow_list_from_deny_toml() {
        assert_eq!(
            allow_list(DENY_TOML).unwrap(),
            ["MIT", "Apache-2.0 WITH LLVM-exception"]
        );
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
