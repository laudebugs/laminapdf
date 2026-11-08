//! `cargo xtask gallery [--test|--update]`: the Paper component gallery (apps/gallery, todo U6f).
//!
//! - no flag: build the packages the gallery reads and serve it with Vite's dev server;
//! - `--test`: run its Playwright tests (screenshots, axe, fonts) in the pinned Playwright image;
//! - `--update`: the same, rewriting `apps/gallery/e2e/__screenshots__/`, then list what changed.
//!
//! The tests run in Docker so that screenshots are Linux Chromium's on every machine. The files the
//! gallery needs (tracked and new, never ignored ones) are streamed into the container, which
//! installs and builds its own copy, so the host's `node_modules` is never touched; results come back
//! through `target/gallery/` (expected, actual and diff images and Playwright's report under
//! `results/`, the new screenshots under `screenshots/`). Only `--update` writes screenshots.

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::step;
use crate::tracked::git;
use crate::workspace::Workspace;

/// The gallery's folder, from the repository root.
const GALLERY: &str = "apps/gallery";
/// Where its screenshots live, from the repository root.
const SCREENSHOTS: &str = "apps/gallery/e2e/__screenshots__";
/// The packages the gallery builds on (their folders, from the repository root).
const PACKAGES: [&str; 5] = [
    "packages/tokens",
    "packages/icons",
    "packages/fonts",
    "packages/ui",
    "packages/test-utils",
];
/// Root files every install needs.
const ROOT_FILES: [&str; 5] = [
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "tsconfig.base.json",
    "biome.json",
];
/// Proxy settings passed into the container when the host has them.
const PROXY_VARS: [&str; 6] = [
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "NO_PROXY",
    "https_proxy",
    "http_proxy",
    "no_proxy",
];

/// What the command does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Serve,
    Test,
    Update,
}

/// Parses `cargo xtask gallery`'s arguments.
pub(crate) fn parse_args(args: &[String]) -> Result<Mode> {
    match args {
        [] => Ok(Mode::Serve),
        [flag] if flag == "--test" => Ok(Mode::Test),
        [flag] if flag == "--update" => Ok(Mode::Update),
        _ => bail!("usage: cargo xtask gallery [--test|--update]"),
    }
}

/// Runs `mode`.
pub(crate) fn run(ws: &Workspace, mode: Mode) -> Result<()> {
    if mode == Mode::Serve {
        step("gallery: build tokens, icons and fonts, then serve the gallery (Vite)");
        build_packages(ws)?;
        return ws.pnpm(&["--filter", "@laminapdf/gallery", "run", "dev"]);
    }
    let image = image(&ws.root)?;
    require_docker()?;
    let out = ws.root.join("target/gallery");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("emptying {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;
    step(&format!(
        "gallery: Playwright tests in {image}{}",
        if mode == Mode::Update {
            ", rewriting the screenshots"
        } else {
            ""
        }
    ));
    let passed = in_container(ws, &image, &out, mode)?;
    if mode == Mode::Update {
        replace_screenshots(&ws.root, &out.join("screenshots"))?;
        let changed = git(&ws.root, &["status", "--porcelain", "--", SCREENSHOTS])?;
        if changed.trim().is_empty() {
            println!("No screenshot changed.");
        } else {
            println!("Changed screenshots (review them in the PR's image diff):\n{changed}");
        }
    }
    if !passed {
        bail!(
            "the gallery's tests failed: expected, actual and diff images and Playwright's report are in {}",
            out.join("results").display()
        );
    }
    Ok(())
}

fn build_packages(ws: &Workspace) -> Result<()> {
    ws.pnpm(&[
        "--filter",
        "@laminapdf/tokens",
        "--filter",
        "@laminapdf/icons",
        "--filter",
        "@laminapdf/fonts",
        "run",
        "build",
    ])
}

/// The Playwright image pinned in apps/gallery/package.json (`config.playwrightImage`).
fn image(root: &Path) -> Result<String> {
    let path = root.join(GALLERY).join("package.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?)?;
    manifest
        .pointer("/config/playwrightImage")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{} has no config.playwrightImage", path.display()))
}

fn require_docker() -> Result<()> {
    let ok = Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        bail!(
            "the gallery's tests run in Docker, which isn't running here. Start Docker, or run the \
             \"Update gallery screenshots\" workflow from the Actions tab, which uploads the new \
             screenshots as an artifact (apps/gallery/README.md)"
        );
    }
    Ok(())
}

/// The files the container needs, `/`-separated from the root: the root files, every workspace
/// package's manifest, and everything tracked or new (not ignored) in the gallery and its packages.
fn files(root: &Path) -> Result<Vec<String>> {
    let mut paths: Vec<&str> = vec![GALLERY];
    paths.extend(PACKAGES);
    let mut args = vec!["ls-files", "-z", "-co", "--exclude-standard", "--"];
    args.extend(paths);
    let listing = git(root, &args)?;
    let mut files: Vec<String> = listing
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    let manifests = git(
        root,
        &["ls-files", "-z", "--", "packages/*/package.json", "apps/*/package.json"],
    )?;
    files.extend(manifests.split('\0').filter(|p| !p.is_empty()).map(str::to_owned));
    files.extend(ROOT_FILES.iter().map(|f| (*f).to_owned()));
    files.retain(|f| root.join(f).is_file());
    files.sort();
    files.dedup();
    Ok(files)
}

/// The script the container runs: unpack, install, build, test, and copy the results out.
fn script(pnpm: &str, mode: Mode) -> String {
    let update = if mode == Mode::Update {
        " --update-snapshots=all"
    } else {
        ""
    };
    let copy = if mode == Mode::Update {
        "cp -r e2e/__screenshots__ /host/screenshots 2>/dev/null || mkdir -p /host/screenshots\n"
    } else {
        ""
    };
    format!(
        "set -e\n\
         mkdir -p /tmp/work && tar -xf - -C /tmp/work && cd /tmp/work\n\
         npm install --global --silent {pnpm}\n\
         CI=true pnpm install --frozen-lockfile --filter '@laminapdf/gallery...'\n\
         pnpm --filter @laminapdf/tokens --filter @laminapdf/icons --filter @laminapdf/fonts run build\n\
         cd apps/gallery\n\
         status=0\n\
         GALLERY_RESULTS=/host/results pnpm exec playwright test{update} || status=$?\n\
         {copy}exit $status\n"
    )
}

/// Runs the tests in `image`, results into `out`; whether they passed.
fn in_container(ws: &Workspace, image: &str, out: &Path, mode: Mode) -> Result<bool> {
    let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(ws.root.join("package.json"))?)?;
    let pnpm = manifest
        .get("packageManager")
        .and_then(serde_json::Value::as_str)
        .context("package.json has no packageManager")?;
    let mut tar = Command::new("tar")
        .args(["-cf", "-", "--null", "-T", "-"])
        .current_dir(&ws.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("running tar")?;
    {
        let mut stdin = tar.stdin.take().context("tar's stdin")?;
        for file in files(&ws.root)? {
            stdin.write_all(file.as_bytes())?;
            stdin.write_all(b"\0")?;
        }
    }
    let archive = tar.stdout.take().context("tar's stdout")?;
    let mut docker = Command::new("docker");
    docker
        .args(["run", "--rm", "-i", "--ipc=host"])
        .arg("-v")
        .arg(format!("{}:/host", out.display()));
    let mut proxied = false;
    for var in PROXY_VARS {
        if let Ok(value) = std::env::var(var) {
            docker.arg("-e").arg(format!("{var}={value}"));
            proxied = true;
        }
    }
    if let Ok(ca) = std::env::var("NODE_EXTRA_CA_CERTS") {
        docker.arg("-v").arg(format!("{ca}:/tmp/extra-ca.crt:ro")).args([
            "-e",
            "NODE_EXTRA_CA_CERTS=/tmp/extra-ca.crt",
            "-e",
            "npm_config_cafile=/tmp/extra-ca.crt",
        ]);
    }
    if proxied {
        // A proxy on the host's loopback is reachable only from the host's network.
        docker.args(["--network", "host"]);
    }
    let status = docker
        .arg(image)
        .args(["sh", "-c", &script(pnpm, mode)])
        .stdin(Stdio::from(archive))
        .status()
        .context("running docker")?;
    let tarred = tar.wait()?;
    if !tarred.success() {
        bail!("tar failed with {tarred}");
    }
    Ok(status.success())
}

/// Replaces the committed screenshots with those the container wrote.
fn replace_screenshots(root: &Path, from: &Path) -> Result<()> {
    let to = root.join(SCREENSHOTS);
    if to.exists() {
        fs::remove_dir_all(&to)?;
    }
    fs::create_dir_all(&to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        fs::copy(entry.path(), to.join(entry.file_name()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_should_read_each_mode() {
        assert_eq!(parse_args(&[]).ok(), Some(Mode::Serve));
        assert_eq!(parse_args(&["--test".to_owned()]).ok(), Some(Mode::Test));
        assert_eq!(parse_args(&["--update".to_owned()]).ok(), Some(Mode::Update));
        assert!(parse_args(&["--other".to_owned()]).is_err());
    }

    #[test]
    fn script_should_write_screenshots_only_when_updating() {
        let test = script("pnpm@11.20.0", Mode::Test);
        assert!(!test.contains("--update-snapshots"));
        assert!(!test.contains("/host/screenshots"));
        let update = script("pnpm@11.20.0", Mode::Update);
        assert!(update.contains("playwright test --update-snapshots=all"));
        assert!(update.contains("/host/screenshots"));
        assert!(update.contains("npm install --global --silent pnpm@11.20.0"));
    }
}
