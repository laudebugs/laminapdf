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
    step("gallery: is Docker running?");
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
    let mut info = Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    // A stalled Docker Desktop never answers `docker info`; give it a while, then say so.
    let answered = match &mut info {
        Ok(child) => wait_up_to(child, DOCKER_ANSWER)?,
        Err(_) => Some(false),
    };
    match answered {
        Some(true) => Ok(()),
        Some(false) => bail!(
            "the gallery's tests run in Docker, which isn't running here. Start Docker, or ask \
             someone with Docker to run this on your branch (apps/gallery/README.md)"
        ),
        None => bail!(
            "Docker didn't answer `docker info` within {} s: Docker Desktop may be starting, paused \
             or stuck. Restart it, wait for \"Engine running\", then run this again",
            DOCKER_ANSWER.as_secs()
        ),
    }
}

/// How long `docker info` may take before Docker counts as stuck.
const DOCKER_ANSWER: std::time::Duration = std::time::Duration::from_secs(20);

/// Waits up to `limit` for `child`: whether it succeeded, or `None` (killed) when it took longer.
fn wait_up_to(child: &mut std::process::Child, limit: std::time::Duration) -> Result<Option<bool>> {
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status.success()));
        }
        if start.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
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
    files.retain(|f| {
        let name = Path::new(f).file_name().unwrap_or_default();
        !is_apple_double(name) && root.join(f).is_file()
    });
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
         trap 'chown -R \"$HOST_OWNER\" /host 2>/dev/null || true' EXIT\n\
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
    // The list goes to a file tar reads: writing it to tar's stdin before docker reads tar's
    // stdout would deadlock once the archive filled the pipe.
    let list = out.join("files.txt");
    let mut names = Vec::new();
    for file in files(&ws.root)? {
        names.extend_from_slice(file.as_bytes());
        names.push(0);
    }
    fs::write(&list, names).with_context(|| format!("writing {}", list.display()))?;
    let mut tar = Command::new("tar")
        .args(["-cf", "-", "--null", "-T"])
        .arg(&list)
        .current_dir(&ws.root)
        // macOS's tar adds an `AppleDouble` `._<name>` entry beside every file with extended
        // attributes; unpacked in the container they are stray sources and screenshots.
        .env("COPYFILE_DISABLE", "1")
        .stdout(Stdio::piped())
        .spawn()
        .context("running tar")?;
    let archive = tar.stdout.take().context("tar's stdout")?;
    let mut docker = Command::new("docker");
    docker
        .args(["run", "--rm", "-i", "--ipc=host"])
        .arg("-v")
        .arg(format!("{}:/host", out.display()))
        // The store and npm's cache outlive the container, so later runs download nothing new.
        .args([
            "-v",
            "lamina-gallery-pnpm-store:/pnpm-store",
            "-e",
            "npm_config_store_dir=/pnpm-store",
        ])
        .args(["-v", "lamina-gallery-npm-cache:/root/.npm"])
        // The container runs as root; what it writes under /host goes back to whoever owns `out`.
        .arg("-e")
        .arg(format!("HOST_OWNER={}", owner(out)));
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
    // A container that failed to start closes tar's pipe; its own message says why, not tar's.
    if status.success() && !tarred.success() {
        bail!("tar failed with {tarred}");
    }
    Ok(status.success())
}

/// `uid:gid` of `path`'s owner (`0:0` where there are none).
fn owner(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if let Ok(meta) = fs::metadata(path) {
            return format!("{}:{}", meta.uid(), meta.gid());
        }
    }
    let _ = path;
    "0:0".to_owned()
}

/// Whether `name` is a macOS `AppleDouble` file (`._<name>`): metadata, never a screenshot.
fn is_apple_double(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b"._")
}

/// Replaces the committed screenshots with those the container wrote; leaves them alone when it
/// wrote none (it failed before its tests ran).
fn replace_screenshots(root: &Path, from: &Path) -> Result<()> {
    let written = fs::read_dir(from).is_ok_and(|mut d| d.next().is_some());
    if !written {
        bail!(
            "the container wrote no screenshots (it failed before its tests ran: see its output above); {SCREENSHOTS} is unchanged"
        );
    }
    let to = root.join(SCREENSHOTS);
    if to.exists() {
        fs::remove_dir_all(&to)?;
    }
    fs::create_dir_all(&to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        if is_apple_double(&entry.file_name()) {
            continue;
        }
        fs::copy(entry.path(), to.join(entry.file_name()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_apple_double_should_match_only_dot_underscore_names() {
        assert!(is_apple_double(std::ffi::OsStr::new("._icons--dark.png")));
        assert!(!is_apple_double(std::ffi::OsStr::new("icons--dark.png")));
        assert!(!is_apple_double(std::ffi::OsStr::new(".gitignore")));
    }

    #[test]
    fn replace_screenshots_should_leave_out_apple_double_files() -> Result<()> {
        let root = std::env::temp_dir().join(format!("lamina-gallery-test-{}", std::process::id()));
        let from = root.join("written");
        fs::create_dir_all(&from)?;
        fs::write(from.join("icons--dark.png"), b"png")?;
        fs::write(from.join("._icons--dark.png"), b"meta")?;
        replace_screenshots(&root, &from)?;
        let mut copied: Vec<_> = fs::read_dir(root.join(SCREENSHOTS))?
            .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_, _>>()?;
        copied.sort();
        fs::remove_dir_all(&root)?;
        assert_eq!(copied, ["icons--dark.png"]);
        Ok(())
    }

    // `true` and `sleep` are Unix commands.
    #[cfg(unix)]
    #[test]
    fn wait_up_to_should_tell_a_finished_process_from_one_that_hangs() -> Result<()> {
        let mut quick = Command::new("true").spawn()?;
        assert_eq!(wait_up_to(&mut quick, std::time::Duration::from_secs(5))?, Some(true));
        let mut slow = Command::new("sleep").arg("30").spawn()?;
        assert_eq!(wait_up_to(&mut slow, std::time::Duration::from_millis(200))?, None);
        Ok(())
    }

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
