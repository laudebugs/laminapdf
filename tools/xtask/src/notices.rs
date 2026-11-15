//! Third-party notices from cargo-about (pinned in doctor.rs), offline: the CLI's archives
//! (`cargo xtask cli-dist`) and the desktop app (`cargo xtask desktop-notices`). Each crate keeps
//! its own `about/about.toml` (the licences it may contain) and `about/notices.hbs`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::workspace::Workspace;

/// Writes `out` for the crate in `dir` with `features` on, from its `about/` files.
pub(crate) fn generate(ws: &Workspace, dir: &Path, features: &[&str], out: &Path) -> Result<()> {
    crate::doctor::require(ws, &crate::doctor::cargo_about())?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let out_arg = out.to_string_lossy().into_owned();
    let joined = features.join(",");
    let mut args = vec![
        "about",
        "generate",
        "--locked",
        "--offline",
        "--fail",
        "--config",
        "about/about.toml",
    ];
    if !features.is_empty() {
        args.extend(["--features", joined.as_str()]);
    }
    args.extend(["about/notices.hbs", "--output-file", out_arg.as_str()]);
    crate::run(dir, "cargo", &args)
}

/// `cargo xtask desktop-notices [--bundle]`: the desktop app's `THIRD_PARTY_NOTICES.md` in
/// target/desktop/, with the MPL-2.0 text that Tauri's MPL-2.0 dependencies require (ADR 0008 §1,
/// ADR 0019). `--bundle` adds the frontend's npm packages and writes the whole file where
/// `tauri.release.conf.json` bundles it from, `src-tauri/resources/THIRD_PARTY_NOTICES.txt`
/// (git-ignored), for Help ▸ View licences.
pub(crate) fn desktop(ws: &Workspace, args: &[String]) -> Result<()> {
    let bundle = match args {
        [] => false,
        [flag] if flag == "--bundle" => true,
        _ => bail!("usage: cargo xtask desktop-notices [--bundle]"),
    };
    let notices = ws.root.join("target/desktop/THIRD_PARTY_NOTICES.md");
    let app = ws.root.join("apps/desktop/src-tauri");
    generate(ws, &app, &[], &notices)?;
    println!("desktop-notices: third-party notices in {}", notices.display());
    if bundle {
        let mut text = fs::read_to_string(&notices).with_context(|| format!("reading {}", notices.display()))?;
        text.push_str(&npm_notices(ws)?);
        let out = app.join("resources/THIRD_PARTY_NOTICES.txt");
        fs::create_dir_all(app.join("resources")).context("creating src-tauri/resources")?;
        fs::write(&out, text).with_context(|| format!("writing {}", out.display()))?;
        println!(
            "desktop-notices: with the npm packages, for the bundle, in {}",
            out.display()
        );
    }
    Ok(())
}

/// One npm package the frontend bundles.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Package {
    name: String,
    version: String,
    path: PathBuf,
}

/// Every package in `pnpm list --json`'s tree below the root, once each; Lamina PDF's own
/// workspace packages are walked through but not listed.
fn packages(list: &Value) -> BTreeSet<Package> {
    fn walk(deps: Option<&Value>, seen: &mut BTreeSet<String>, found: &mut BTreeSet<Package>) {
        let Some(deps) = deps.and_then(Value::as_object) else {
            return;
        };
        for (name, info) in deps {
            let path = info.get("path").and_then(Value::as_str).unwrap_or_default();
            // pnpm lists a package's dependencies only at its first appearance; walk each path once.
            if !seen.insert(path.to_owned()) {
                continue;
            }
            let version = info.get("version").and_then(Value::as_str).unwrap_or_default();
            if !name.starts_with("@laminapdf/") {
                found.insert(Package {
                    name: name.clone(),
                    version: version.to_owned(),
                    path: PathBuf::from(path),
                });
            }
            walk(info.get("dependencies"), seen, found);
        }
    }
    let mut seen = BTreeSet::new();
    let mut found = BTreeSet::new();
    let roots = list.as_array().into_iter().flatten();
    for root in roots {
        walk(root.get("dependencies"), &mut seen, &mut found);
    }
    found
}

/// The files in `dir` whose names `keep` accepts, by name, with their text.
fn text_files(dir: &Path, keep: impl Fn(&str) -> bool) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        let name = crate::util::file_name(&path);
        if path.is_file() && keep(&name) {
            files.insert(
                name,
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
            );
        }
    }
    Ok(files)
}

/// Whether `name` is a licence file's: LICENSE, LICENCE, COPYING or NOTICE, any case or extension.
fn is_licence_file(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["LICENSE", "LICENCE", "COPYING", "NOTICE"]
        .iter()
        .any(|p| upper.starts_with(p))
}

/// The notices of the frontend's npm packages and of the fonts it bundles.
fn npm_notices(ws: &Workspace) -> Result<String> {
    let listed = crate::util::stdout(
        "pnpm",
        &[
            "--dir",
            &ws.root.to_string_lossy(),
            "--filter",
            "@laminapdf/desktop",
            "list",
            "--prod",
            "--depth",
            "Infinity",
            "--json",
        ],
    )
    .context("pnpm list failed: run `pnpm install` first")?;
    let list: Value = serde_json::from_str(&listed).context("reading pnpm list's JSON")?;
    let mut text = String::from(
        "\n# JavaScript packages\n\nThe app's window is built from these npm packages, with the licence each names and the \
         licence files it ships.\n",
    );
    for package in packages(&list) {
        let manifest_path = package.path.join("package.json");
        let manifest =
            fs::read_to_string(&manifest_path).with_context(|| format!("reading {}", manifest_path.display()))?;
        let manifest: Value =
            serde_json::from_str(&manifest).with_context(|| format!("parsing {}", manifest_path.display()))?;
        let licence = manifest.get("license").and_then(Value::as_str).unwrap_or("not stated");
        let files = text_files(&package.path, is_licence_file)?;
        write!(
            text,
            "\n## {} {}\n\nLicence: {licence}\n",
            package.name, package.version
        )?;
        if files.is_empty() {
            text.push_str("\nThe package ships no licence file; its package.json names the licence above.\n");
        }
        for (name, body) in files {
            write!(text, "\n{name}:\n\n```text\n{}\n```\n", body.trim_end())?;
        }
    }
    text.push_str("\n# Fonts\n\nThe app's typefaces (`@laminapdf/fonts`) are under the SIL Open Font License 1.1.\n");
    let fonts = ws.root.join("packages/fonts/LICENSES");
    let licences = text_files(&fonts, |_| true)?;
    if licences.is_empty() {
        bail!("no font licence in {}", fonts.display());
    }
    for (name, body) in licences {
        write!(text, "\n## {name}\n\n```text\n{}\n```\n", body.trim_end())?;
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licence_files_should_match_any_case_and_extension() {
        for name in ["LICENSE", "license.md", "LICENCE-MIT", "COPYING", "NOTICE.txt"] {
            assert!(is_licence_file(name), "{name}");
        }
        assert!(!is_licence_file("README.md") && !is_licence_file("package.json"));
    }

    #[test]
    fn packages_should_list_each_dependency_once_and_skip_our_own() {
        let list = serde_json::json!([{
            "dependencies": {
                "@laminapdf/ui": { "version": "link:../../packages/ui", "path": "/w/packages/ui", "dependencies": {
                    "react-aria": { "version": "3.53.1", "path": "/s/react-aria", "dependencies": {
                        "clsx": { "version": "2.1.1", "path": "/s/clsx" }
                    }},
                    "react": { "version": "19.3.1", "path": "/s/react" }
                }},
                "react": { "version": "19.3.1", "path": "/s/react" }
            }
        }]);
        let names: Vec<String> = packages(&list).into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["clsx", "react", "react-aria"]);
    }
}
