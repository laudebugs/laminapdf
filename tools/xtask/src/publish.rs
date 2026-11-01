//! `cargo xtask publish crates|npm [--dry-run]` and `cargo xtask sbom <dir>`: the release lane's
//! publish jobs (ADR 0017 §7, todo P1).
//!
//! - `crates`: `cargo publish --workspace --locked`, which uploads every crate whose manifest says
//!   `publish = true`, in dependency order, each verified by a build from its packaged files. A real
//!   publish needs `CARGO_REGISTRY_TOKEN`, which the workflow gets from crates.io's trusted
//!   publishing (`rust-lang/crates-io-auth-action`); no crates.io token is stored anywhere. A
//!   re-run after a partial publish leaves out the crates crates.io already has at the version,
//!   and `npm-stage` the packages npm has.
//! - `npm-pack`: builds the four packages `cargo xtask wasm` doesn't (render, render-pdfium,
//!   render-hayro, viewer) and packs each member of `.changeset/config.json`'s fixed group with
//!   `pnpm pack` (which turns `workspace:*` into the version), with no token in reach;
//!   `npm-stage` stages each tarball, dependencies first, with `npm stage publish` (npm
//!   11.17.0 through `npx`); `npm` does both, with public access and the version's dist-tag (`alpha` for
//!   `0.2.0-alpha.1`, else `latest`). A staged version is published only when a maintainer
//!   approves it on npmjs.com with 2FA. A real run needs `NPM_TOKEN`, a granular token limited to
//!   `@laminapdf` that can stage but not publish (npm's trusted publishing rejects self-hosted
//!   runners). The token reaches npm through a temporary `.npmrc` that names the variable, so it
//!   is never written to disk or printed.
//! - `sbom`: `CycloneDX` SBOMs of every publishable crate (`cargo cyclonedx`) and npm package (`pnpm
//!   sbom`) into `<dir>`, for the GitHub release.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// What `cargo xtask publish` uploads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Registry {
    Crates,
    /// Pack and stage (`NpmPack` then `NpmStage`).
    Npm,
    /// Build and pack the fixed group, needing no token.
    NpmPack,
    /// Stage the packed tarballs: the only step that sees `NPM_TOKEN`.
    NpmStage,
}

/// `cargo xtask publish <crates|npm> [--dry-run]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Publish {
    pub(crate) registry: Registry,
    pub(crate) dry_run: bool,
}

const USAGE: &str = "usage: cargo xtask publish <crates|npm|npm-pack|npm-stage> [--dry-run]";

/// Parses `publish`'s arguments.
pub(crate) fn parse_args(args: &[String]) -> Result<Publish> {
    let (registry, flags) = match args {
        [registry, flags @ ..] if registry == "crates" => (Registry::Crates, flags),
        [registry, flags @ ..] if registry == "npm" => (Registry::Npm, flags),
        [registry, flags @ ..] if registry == "npm-pack" => (Registry::NpmPack, flags),
        [registry, flags @ ..] if registry == "npm-stage" => (Registry::NpmStage, flags),
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
        Registry::Npm => {
            npm_pack(ws)?;
            npm_stage(ws, publish.dry_run)
        }
        Registry::NpmPack => npm_pack(ws),
        Registry::NpmStage => npm_stage(ws, publish.dry_run),
    }
}

fn crates(ws: &Workspace, dry_run: bool) -> Result<()> {
    if !dry_run && std::env::var_os("CARGO_REGISTRY_TOKEN").is_none() {
        bail!("CARGO_REGISTRY_TOKEN is not set: the release workflow gets it from crates.io's trusted publishing");
    }
    let version = crate::version::fixed_group_version(&ws.root)?.version;
    let mut args = vec!["publish".to_owned(), "--workspace".to_owned(), "--locked".to_owned()];
    if dry_run {
        args.push("--dry-run".to_owned());
    } else {
        // A re-run after a partial publish: leave out what crates.io already has.
        for name in ws.publishable_crates() {
            if on_crates_io(name, &version)? {
                println!("{name} {version} is on crates.io already: skipped");
                args.extend(["--exclude".to_owned(), name.to_owned()]);
            }
        }
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    ws.cargo(&args)
}

/// Whether crates.io has `name` at `version` (its API answers 200 for a published version).
fn on_crates_io(name: &str, version: &str) -> Result<bool> {
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

/// Where `npm-pack` leaves the tarballs and `npm-stage` reads them.
const PACKS: &str = "target/xtask/npm-packs";
/// The npm CLI that stages: `npm stage` needs 11.15 or later, newer than the npm some Node 24
/// releases bundle, so it runs through `npx` at this version.
const NPM_VERSION: &str = "11.17.0";

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
    Ok(())
}

fn npm_stage(ws: &Workspace, dry_run: bool) -> Result<()> {
    let group = crate::version::fixed_group_version(&ws.root)?;
    let tag = dist_tag(&group.version);
    let npmrc = if dry_run {
        None
    } else {
        if std::env::var_os("NPM_TOKEN").is_none() {
            bail!(
                "NPM_TOKEN is not set: a granular npm token limited to @laminapdf, stored as the repository secret NPM_TOKEN"
            );
        }
        Some(NpmRc::write(&ws.root)?)
    };
    let npm = format!("npm@{NPM_VERSION}");
    for name in dependency_order(&group.members)? {
        let tarball = ws.root.join(PACKS).join(tarball_name(&name, &group.version));
        if !tarball.is_file() {
            bail!(
                "{} is missing: run `cargo xtask publish npm-pack` first",
                tarball.display()
            );
        }
        if !dry_run && on_npm(ws, &name, &group.version)? {
            println!("{name}@{} is on npm already: skipped", group.version);
            continue;
        }
        let tarball = tarball.to_str().context("a tarball path is not UTF-8")?;
        let mut args = vec!["--yes", npm.as_str()];
        args.extend(stage_args(tarball, tag, dry_run));
        match &npmrc {
            Some(npmrc) => crate::run_env(&ws.root, "npx", &args, &[("NPM_CONFIG_USERCONFIG", npmrc.path_str()?)])?,
            None => crate::run(&ws.root, "npx", &args)?,
        }
    }
    Ok(())
}

/// Whether npm has published `name` at `version` (`npm view` prints the version, else nothing).
/// A version staged but not yet approved is not on npm: stage it again or approve the staged one.
fn on_npm(ws: &Workspace, name: &str, version: &str) -> Result<bool> {
    let output = std::process::Command::new(crate::program_path("npm"))
        .args(["view", &format!("{name}@{version}"), "version"])
        .current_dir(&ws.root)
        .output()
        .context("running npm view")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim() == version)
}

/// The members' names with every package after the members it depends on, so they are staged,
/// and approved, dependencies first.
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

/// `npm stage publish`'s arguments for one tarball.
fn stage_args<'a>(tarball: &'a str, tag: &'a str, dry_run: bool) -> Vec<&'a str> {
    let mut args = vec!["stage", "publish", tarball, "--access", "public", "--tag", tag];
    if dry_run {
        args.push("--dry-run");
    }
    args
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

/// A temporary user `.npmrc` that reads the token from `NPM_TOKEN` (npm expands `${…}` in it),
/// removed when dropped.
struct NpmRc(PathBuf);

impl NpmRc {
    fn write(root: &Path) -> Result<Self> {
        let path = root.join("target/xtask/publish.npmrc");
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        fs::write(&path, "//registry.npmjs.org/:_authToken=${NPM_TOKEN}\n")
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(Self(path))
    }

    fn path_str(&self) -> Result<&str> {
        self.0.to_str().context("the .npmrc path is not UTF-8")
    }
}

impl Drop for NpmRc {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
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
            parse_args(&strings(&["npm"])).unwrap(),
            Publish {
                registry: Registry::Npm,
                dry_run: false
            }
        );
        assert!(parse_args(&strings(&["pypi"])).is_err());
        assert!(parse_args(&strings(&["npm", "--force"])).is_err());
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
    fn stage_args_should_stage_the_tarball_publicly_with_the_tag() {
        assert_eq!(
            stage_args("x.tgz", "alpha", true),
            [
                "stage",
                "publish",
                "x.tgz",
                "--access",
                "public",
                "--tag",
                "alpha",
                "--dry-run"
            ]
        );
        assert_eq!(stage_args("x.tgz", "latest", false).last(), Some(&"latest"));
    }
}
