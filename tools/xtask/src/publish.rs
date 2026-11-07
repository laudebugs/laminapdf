//! The release lane's publish jobs (ADR 0017 §7): `cargo xtask publish crates [--dry-run]`,
//! `cargo xtask publish npm-pack` and `cargo xtask sbom <dir>`.
//!
//! - `crates`: `cargo publish --workspace --locked`, which uploads every crate whose manifest says
//!   `publish = true`, in dependency order, each verified by a build from its packaged files. A real
//!   publish needs `CARGO_REGISTRY_TOKEN`, which the workflow gets from crates.io's trusted
//!   publishing (`rust-lang/crates-io-auth-action`); no crates.io token is stored anywhere. A
//!   re-run after a partial publish leaves out the crates crates.io already has at the version.
//! - `npm-pack`: builds the four packages `cargo xtask wasm` doesn't (render, render-pdfium,
//!   render-hayro, viewer), packs each member of `.changeset/config.json`'s fixed group with
//!   `pnpm pack` (which turns `workspace:*` into the version), and writes `publish.json` beside
//!   the tarballs: the version, its dist-tag (`alpha` for `0.2.0-alpha.1`, else `latest`) and the
//!   tarballs in dependency order. npm's trusted publishing accepts
//!   only cloud-hosted runners, so the release lane's `publish-npm` job, on a GitHub-hosted runner
//!   with an OIDC token and no checkout, reads that file and runs `npm publish` for each tarball;
//!   no npm token is stored, and nothing here publishes.
//! - `sbom`: `CycloneDX` SBOMs of every publishable crate (`cargo cyclonedx`) and npm package (`pnpm
//!   sbom`) into `<dir>`, for the GitHub release.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// What `cargo xtask publish` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Registry {
    Crates,
    /// Build and pack the fixed group and write `publish.json`, for the GitHub-hosted job that
    /// publishes to npm.
    NpmPack,
}

/// `cargo xtask publish <crates|npm-pack> [--dry-run]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Publish {
    pub(crate) registry: Registry,
    pub(crate) dry_run: bool,
}

const USAGE: &str = "usage: cargo xtask publish <crates|npm-pack> [--dry-run]";

/// Parses `publish`'s arguments.
pub(crate) fn parse_args(args: &[String]) -> Result<Publish> {
    let (registry, flags) = match args {
        [registry, flags @ ..] if registry == "crates" => (Registry::Crates, flags),
        [registry, flags @ ..] if registry == "npm-pack" => (Registry::NpmPack, flags),
        _ => bail!("{USAGE}"),
    };
    match flags {
        [] => Ok(Publish {
            registry,
            dry_run: false,
        }),
        [flag] if flag == "--dry-run" => Ok(Publish {
            registry,
            dry_run: true,
        }),
        _ => bail!("{USAGE}"),
    }
}

/// Runs a publish.
pub(crate) fn run(ws: &Workspace, publish: Publish) -> Result<()> {
    match publish.registry {
        Registry::Crates => crates(ws, publish.dry_run),
        Registry::NpmPack => npm_pack(ws),
    }
}

fn crates(ws: &Workspace, dry_run: bool) -> Result<()> {
    if !dry_run && std::env::var_os("CARGO_REGISTRY_TOKEN").is_none() {
        bail!("CARGO_REGISTRY_TOKEN is not set: the release workflow gets it from crates.io's trusted publishing");
    }
    clean_for_publish(&ws.root)?;
    let version = crate::version::fixed_group_version(&ws.root)?.version;
    crate_docs(ws)?;
    crate::package_docs::check_crate(ws, &version)?;
    let mut args = cargo_publish_args(dry_run);
    if !dry_run {
        // A re-run after a partial publish: leave out what crates.io already has.
        for name in ws.publishable_crates() {
            if on_crates_io(name, &version)? {
                println!("{name} {version} is on crates.io already: skipped");
                args.extend(["--exclude".to_owned(), name.to_owned()]);
            }
        }
    }
    let excluded = args.iter().filter(|a| *a == "--exclude").count();
    if excluded == ws.publishable_crates().count() {
        println!("every crate is on crates.io at {version} already: nothing to publish");
        return Ok(());
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    ws.cargo(&args)
}

/// Refuse unrelated changes before generating the ignored package docs that require
/// `--allow-dirty`; otherwise Cargo could include a developer's uncommitted source edits.
fn clean_for_publish(root: &Path) -> Result<()> {
    let status = crate::tracked::git(root, &["status", "--porcelain=v1", "--untracked-files=normal"])?;
    if status.trim().is_empty() {
        Ok(())
    } else {
        bail!("the checkout has changes unrelated to generated package docs:\n{status}")
    }
}

/// The package docs are generated into a gitignored folder immediately before this command and
/// checked against the crate package, so Cargo must accept those intentional uncommitted files.
fn cargo_publish_args(dry_run: bool) -> Vec<String> {
    let mut args = vec![
        "publish".to_owned(),
        "--workspace".to_owned(),
        "--locked".to_owned(),
        "--allow-dirty".to_owned(),
    ];
    if dry_run {
        args.push("--dry-run".to_owned());
    }
    args
}

/// The `laminapdf` crate's package docs (L5a): the site's pages, the code pages, `AGENTS.md` and
/// `renames.json` as Markdown and JSON in `crates/laminapdf/docs/` (gitignored), which its
/// `include` ships. Plain Node, no packages: the API reference is the npm package's only.
fn crate_docs(ws: &Workspace) -> Result<()> {
    crate::step("package docs: crates/laminapdf/docs/");
    let node = std::process::Command::new(crate::program_path("node"))
        .arg("--version")
        .output();
    if !node.is_ok_and(|out| out.status.success()) {
        bail!(
            "node is not on PATH: `publish crates` writes the laminapdf crate's docs/ with Node \
             (in release.yml, give the publish-crates job `./.github/actions/node-pnpm` with `install: \"\"`)"
        );
    }
    crate::run(
        &ws.root,
        "node",
        &["apps/docs/scripts/package-docs.mjs", "crates/laminapdf/docs"],
    )
}

/// Whether crates.io has `name` at `version` (its API answers 200 for a published version).
pub(crate) fn on_crates_io(name: &str, version: &str) -> Result<bool> {
    let url = format!("https://crates.io/api/v1/crates/{name}/{version}");
    let status = std::process::Command::new(crate::program_path("curl"))
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-A",
            "laminapdf-release (cargo xtask publish)",
            &url,
        ])
        .output()
        .with_context(|| format!("running curl for {url}"))?;
    match String::from_utf8_lossy(&status.stdout).trim() {
        "200" => Ok(true),
        "404" => Ok(false),
        other => bail!("crates.io answered {other} for {url}"),
    }
}

/// The files `cargo xtask wasm` leaves, which the npm packages ship.
const BUILT_BY_WASM: [&str; 2] = ["packages/wasm/dist/lamina_pdf_base.wasm", "packages/sdk/dist/index.js"];

/// Where `npm-pack` leaves the tarballs and `publish.json`.
const PACKS: &str = "target/xtask/npm-packs";
/// The file beside the tarballs that tells the publish job what to publish, in which order.
const MANIFEST: &str = "publish.json";

fn npm_pack(ws: &Workspace) -> Result<()> {
    if let Some(missing) = BUILT_BY_WASM.iter().find(|path| !ws.root.join(path).is_file()) {
        bail!("{missing} is missing: run `cargo xtask wasm` (or download its artifact) first");
    }
    let group = crate::version::fixed_group_version(&ws.root)?;
    // The viewer's build builds render, render-pdfium and render-hayro first.
    ws.pnpm(&["--filter", "@laminapdf/viewer", "run", "build:wasm"])?;
    let packs = ws.root.join(PACKS);
    let _ = fs::remove_dir_all(&packs);
    fs::create_dir_all(&packs).with_context(|| format!("creating {}", packs.display()))?;
    let packs_dir = packs.to_str().context("the pack folder is not UTF-8")?;
    for (name, _) in &group.members {
        ws.pnpm(&["--filter", name, "pack", "--pack-destination", packs_dir])?;
    }
    let sdk = packs.join(tarball_name("@laminapdf/sdk", &group.version));
    crate::package_docs::check_npm_tarball(ws, &sdk, &group.version)?;
    let order = dependency_order(&group.members)?;
    let manifest = publish_manifest(&group.version, &order);
    let path = packs.join(MANIFEST);
    let text = serde_json::to_string_pretty(&manifest).context("writing publish.json")?;
    fs::write(&path, format!("{text}\n")).with_context(|| format!("writing {}", path.display()))?;
    println!(
        "{}: {} packages at {} under the `{}` tag, dependencies first",
        path.display(),
        order.len(),
        group.version,
        dist_tag(&group.version)
    );
    Ok(())
}

/// What the publish job reads (ADR 0017 §7): the version, its dist-tag, and each package's name
/// and tarball in `names`' order, dependencies first. The job runs no code from the repository, so
/// the order and the tag come from here; it checks every value before it uses one, and pins npm
/// itself.
fn publish_manifest(version: &str, names: &[String]) -> serde_json::Value {
    let packages: Vec<serde_json::Value> = names
        .iter()
        .map(|name| serde_json::json!({ "name": name, "tarball": tarball_name(name, version) }))
        .collect();
    serde_json::json!({
        "version": version,
        "tag": dist_tag(version),
        "packages": packages,
    })
}

/// Whether npm has published `name` at `version` (`npm view` prints the version, else nothing).
pub(crate) fn on_npm(ws: &Workspace, name: &str, version: &str) -> Result<bool> {
    let output = std::process::Command::new(crate::program_path("npm"))
        .args(["view", &format!("{name}@{version}"), "version"])
        .current_dir(&ws.root)
        .output()
        .context("running npm view")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim() == version)
}

/// The members' names with every package after the members it depends on, so they are published
/// dependencies first.
fn dependency_order(members: &[(String, PathBuf)]) -> Result<Vec<String>> {
    let mut deps = Vec::with_capacity(members.len());
    for (name, dir) in members {
        let path = dir.join("package.json");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let names: Vec<String> = ["dependencies", "peerDependencies"]
            .iter()
            .filter_map(|key| manifest.get(*key).and_then(serde_json::Value::as_object))
            .flat_map(|table| table.keys().cloned())
            .collect();
        deps.push((name.clone(), names));
    }
    topological(&deps)
}

/// `(name, its dependencies)` sorted so each name follows the listed names it depends on;
/// a cycle is an error.
fn topological(deps: &[(String, Vec<String>)]) -> Result<Vec<String>> {
    let mut order: Vec<String> = Vec::with_capacity(deps.len());
    while order.len() < deps.len() {
        let ready = deps.iter().find(|(name, needs)| {
            !order.contains(name)
                && needs
                    .iter()
                    .all(|need| order.contains(need) || !deps.iter().any(|(n, _)| n == need))
        });
        let Some((name, _)) = ready else {
            bail!("the fixed group's packages depend on each other in a cycle");
        };
        order.push(name.clone());
    }
    Ok(order)
}

/// The file `pnpm pack` writes for `name` at `version`: `@laminapdf/sdk` → `laminapdf-sdk-<version>.tgz`.
fn tarball_name(name: &str, version: &str) -> String {
    format!("{}-{version}.tgz", name.trim_start_matches('@').replace('/', "-"))
}

/// The npm dist-tag for `version`: its pre-release identifier (`alpha` for `0.2.0-alpha.1`;
/// `next` when it is a number), else `latest`, so a pre-release never becomes what
/// `npm install @laminapdf/sdk` picks.
fn dist_tag(version: &str) -> &str {
    match version.split_once('-').and_then(|(_, pre)| pre.split('.').next()) {
        None => "latest",
        Some(id) if id.is_empty() || id.chars().all(|c| c.is_ascii_digit()) => "next",
        Some(id) => id,
    }
}

/// Moves `dir`'s `<name>.cdx.json` into `out` when `name` is publishable, else deletes it.
fn cleanup_sbom(dir: &Path, publishable: &[&str], out: &Path) -> Result<()> {
    let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
        return Ok(());
    };
    let written = dir.join(format!("{name}.cdx.json"));
    if !written.is_file() {
        return Ok(());
    }
    if publishable.contains(&name) {
        fs::rename(&written, out.join(format!("{name}.cdx.json")))
            .with_context(|| format!("moving {} into {}", written.display(), out.display()))
    } else {
        fs::remove_file(&written).with_context(|| format!("removing {}", written.display()))
    }
}

/// `cargo xtask sbom <dir>`: a `CycloneDX` SBOM per publishable crate and per npm package.
pub(crate) fn sbom(ws: &Workspace, out: &Path) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    crate::doctor::require(ws, &crate::doctor::cargo_cyclonedx())?;
    // One run writes `<crate>.cdx.json` into every workspace member's folder; keep the
    // publishable crates' and remove the rest, so the checkout is left as it was.
    ws.cargo(&["cyclonedx", "--format", "json"])?;
    let publishable: Vec<&str> = ws.publishable_crates().collect();
    for parent in ["crates", "tools"] {
        for entry in fs::read_dir(ws.root.join(parent)).with_context(|| format!("reading {parent}/"))? {
            let dir = entry.with_context(|| format!("reading {parent}/"))?.path();
            cleanup_sbom(&dir, &publishable, out)?;
        }
    }
    let group = crate::version::fixed_group_version(&ws.root)?;
    let pattern = out.join("%s-npm.cdx.json");
    let pattern = pattern.to_str().context("the SBOM path is not UTF-8")?;
    let mut args = vec![
        "sbom",
        "--sbom-format",
        "cyclonedx",
        "--prod",
        "--split",
        "--out",
        pattern,
    ];
    for (name, _) in &group.members {
        args.extend(["--filter", name.as_str()]);
    }
    ws.pnpm(&args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_read_the_registry_and_the_dry_run_flag() {
        assert_eq!(
            parse_args(&strings(&["crates", "--dry-run"])).unwrap(),
            Publish {
                registry: Registry::Crates,
                dry_run: true
            }
        );
        assert_eq!(
            parse_args(&strings(&["npm-pack"])).unwrap(),
            Publish {
                registry: Registry::NpmPack,
                dry_run: false
            }
        );
        assert!(parse_args(&strings(&["pypi"])).is_err());
        assert!(parse_args(&strings(&["npm-stage"])).is_err());
        assert!(parse_args(&strings(&["npm-pack", "--force"])).is_err());
    }

    #[test]
    fn cargo_publish_should_allow_the_generated_package_docs() {
        assert_eq!(
            cargo_publish_args(false),
            strings(&["publish", "--workspace", "--locked", "--allow-dirty"])
        );
        assert_eq!(
            cargo_publish_args(true),
            strings(&["publish", "--workspace", "--locked", "--allow-dirty", "--dry-run"])
        );
    }

    #[test]
    fn clean_for_publish_should_refuse_uncommitted_sources() {
        let tree = crate::tracked::TempTree::new("publish-clean", &[("tracked", "before")]);
        crate::tracked::git(&tree.0, &["init", "-q"]).unwrap();
        crate::tracked::git(&tree.0, &["add", "tracked"]).unwrap();
        crate::tracked::git(
            &tree.0,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-qm",
                "initial",
            ],
        )
        .unwrap();
        assert!(clean_for_publish(&tree.0).is_ok());

        fs::write(tree.0.join("tracked"), "after").unwrap();
        assert!(clean_for_publish(&tree.0).is_err());
    }

    #[test]
    fn dist_tag_should_name_the_pre_release_or_latest() {
        assert_eq!(dist_tag("0.2.0-alpha.1"), "alpha");
        assert_eq!(dist_tag("1.0.0-rc.0"), "rc");
        assert_eq!(dist_tag("0.2.0"), "latest");
        assert_eq!(dist_tag("0.2.0-1"), "next");
    }

    #[test]
    fn topological_should_put_dependencies_first_and_refuse_a_cycle() {
        let deps = [
            ("sdk".to_owned(), strings(&["wasm", "left-pad"])),
            ("viewer".to_owned(), strings(&["sdk", "render"])),
            ("wasm".to_owned(), vec![]),
            ("render".to_owned(), vec![]),
        ];
        assert_eq!(
            topological(&deps).unwrap(),
            strings(&["wasm", "sdk", "render", "viewer"])
        );
        let cycle = [("a".to_owned(), strings(&["b"])), ("b".to_owned(), strings(&["a"]))];
        assert!(topological(&cycle).is_err());
    }

    #[test]
    fn tarball_name_should_follow_pnpm_pack() {
        assert_eq!(
            tarball_name("@laminapdf/sdk", "0.2.0-alpha.1"),
            "laminapdf-sdk-0.2.0-alpha.1.tgz"
        );
    }

    #[test]
    fn publish_manifest_should_list_the_tarballs_in_order_with_the_tag() {
        let manifest = publish_manifest("0.2.0-alpha.3", &strings(&["@laminapdf/wasm", "@laminapdf/sdk"]));
        assert_eq!(
            manifest,
            serde_json::json!({
                "version": "0.2.0-alpha.3",
                "tag": "alpha",
                "packages": [
                    { "name": "@laminapdf/wasm", "tarball": "laminapdf-wasm-0.2.0-alpha.3.tgz" },
                    { "name": "@laminapdf/sdk", "tarball": "laminapdf-sdk-0.2.0-alpha.3.tgz" },
                ],
            })
        );
    }

    /// release.yml's `publish-npm` reads publish.json with jq, so its keys must be the ones
    /// [`publish_manifest`] writes: a rename on either side would fail only in a release.
    #[test]
    fn publish_npm_should_read_the_keys_publish_manifest_writes() {
        const WORKFLOW: &str = include_str!("../../../.github/workflows/release.yml");
        let manifest = publish_manifest("0.2.0", &strings(&["@laminapdf/wasm"]));
        let keys = |value: &serde_json::Value| -> Vec<String> {
            value
                .as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default()
        };
        assert_eq!(keys(&manifest), ["packages", "tag", "version"]);
        assert_eq!(keys(&manifest["packages"][0]), ["name", "tarball"]);
        for read in [
            "jq -r .version publish.json",
            "jq -r .tag publish.json",
            r#"jq -r '.packages[] | "\(.name) \(.tarball)"' publish.json"#,
        ] {
            assert!(WORKFLOW.contains(read), "publish-npm no longer reads `{read}`");
        }
    }
}
