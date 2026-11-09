//! What `.changeset/config.json` must say about the workspace's private packages.
//!
//! The config versions private packages (`privatePackages.version`, ADR 0017), so a private
//! package that depends on a member of the fixed group gets bumped whenever the group is, unless
//! it is in `ignore`. The release's `prepare` then refuses the commit, since its changes go beyond
//! the version and the packaged docs: the reader (`apps/web`, which depends on the viewer) did
//! that to the first daily release after the viewer changed. Every private package belongs in
//! `ignore`; `version sync` checks it.

use std::{collections::BTreeSet, fs, path::Path};

use crate::version::{package_folders, read_json};

use anyhow::{Context, Result};
use serde_json::Value;

/// The folders whose packages the pnpm workspace holds (`pnpm-workspace.yaml`).
const WORKSPACE_DIRS: [&str; 2] = ["packages", "apps"];

/// One line per private workspace package missing from `ignore`: what Changesets would version.
pub(crate) fn private_packages_outside_ignore(root: &Path) -> Result<Vec<String>> {
    let config_path = root.join(".changeset/config.json");
    let config: Value = serde_json::from_str(
        &fs::read_to_string(&config_path).with_context(|| format!("reading {}", config_path.display()))?,
    )
    .context(".changeset/config.json is not valid JSON")?;
    let ignored: BTreeSet<&str> = config
        .get("ignore")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut problems = Vec::new();
    for name in private_packages(root)? {
        if !ignored.contains(name.as_str()) {
            problems.push(format!(
                ".changeset/config.json: add the private package {name} to `ignore`, or Changesets versions it \
                 with the fixed group and the release refuses the commit"
            ));
        }
    }
    Ok(problems)
}

/// The names of the workspace's packages whose `package.json` says `"private": true`, sorted. A
/// manifest that doesn't parse can't say, so it is skipped with a warning; `version sync` fails on
/// it when it is a fixed-group member's.
fn private_packages(root: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    // A workspace may lack either folder (the tests' trees have no `apps`).
    for dir in WORKSPACE_DIRS.into_iter().filter(|dir| root.join(dir).is_dir()) {
        for folder in package_folders(root, dir)? {
            let path = folder.join("package.json");
            let package = match read_json(&path) {
                Ok(package) => package,
                Err(error) => {
                    eprintln!("warning: skipping {}: {error:#}", path.display());
                    continue;
                }
            };
            let private = package.get("private").and_then(Value::as_bool) == Some(true);
            if let (true, Some(name)) = (private, package.get("name").and_then(Value::as_str)) {
                names.insert(name.to_owned());
            }
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::private_packages_outside_ignore;
    use crate::tracked::TempTree;

    const CONFIG: &str = r#"{ "fixed": [["@x/sdk"]], "ignore": ["@x/docs"] }"#;

    #[test]
    fn a_private_package_outside_ignore_should_be_reported() {
        let tree = TempTree::new(
            "changesets-private",
            &[
                (".changeset/config.json", CONFIG),
                (
                    "packages/sdk/package.json",
                    r#"{ "name": "@x/sdk", "version": "0.1.0" }"#,
                ),
                ("apps/docs/package.json", r#"{ "name": "@x/docs", "private": true }"#),
                ("apps/web/package.json", r#"{ "name": "@x/web", "private": true }"#),
            ],
        );
        let problems = private_packages_outside_ignore(&tree.0).unwrap();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("@x/web"), "{problems:?}");
    }

    #[test]
    fn every_private_package_in_ignore_should_pass() {
        let tree = TempTree::new(
            "changesets-ignored",
            &[
                (".changeset/config.json", CONFIG),
                ("apps/docs/package.json", r#"{ "name": "@x/docs", "private": true }"#),
                ("packages/sdk/package.json", r#"{ "name": "@x/sdk" }"#),
            ],
        );
        assert_eq!(private_packages_outside_ignore(&tree.0).unwrap(), Vec::<String>::new());
    }
}
