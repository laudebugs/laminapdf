//! `cargo xtask desktop-release …`: the desktop release workflow's steps that make decisions
//! (`.github/workflows/desktop-release.yml`).
//!
//! - `plan`: the run's version, channel, environment and build matrix (`plan.rs`), from the
//!   Actions environment, written to `$GITHUB_OUTPUT` and the job summary.
//! - `sizes <dir>`: every installer under `<dir>` against the desktop plan's budget (< 16 MB,
//!   warning from 14.4 MB), as a table in the job summary.
//! - `sbom <target> <dir>`: `CycloneDX` SBOMs of the app's crates for `<target>` and of the
//!   frontend's npm packages, into `<dir>`.

mod plan;

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::util::{github_output, summary};
use crate::workspace::Workspace;
use plan::{Date, Inputs, Os, Signing, Trigger};

pub(crate) fn run(ws: &Workspace, args: &[String]) -> Result<()> {
    match args {
        [cmd] if cmd == "plan" => run_plan(ws),
        [cmd, dir] if cmd == "sizes" => sizes(Path::new(dir)),
        [cmd, target, dir] if cmd == "sbom" => sbom(ws, target, Path::new(dir)),
        _ => bail!("usage: cargo xtask desktop-release plan | sizes <dir> | sbom <target> <dir>"),
    }
}

fn var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

fn flag(name: &str) -> bool {
    var(name) == "true"
}

/// The trigger the Actions environment describes.
fn trigger() -> Result<Trigger> {
    Ok(match var("GITHUB_EVENT_NAME").as_str() {
        "push" if var("GITHUB_REF_TYPE") == "tag" => Trigger::Tag(var("GITHUB_REF_NAME")),
        "schedule" => Trigger::Schedule,
        "workflow_dispatch" => Trigger::Dispatch {
            channel: var("INPUT_CHANNEL"),
            dry_run: flag("INPUT_DRY_RUN"),
            allow_unsigned: flag("INPUT_ALLOW_UNSIGNED_PLATFORMS"),
            only: Os::parse_only(&var("INPUT_ONLY_OS"))?,
        },
        "pull_request" => Trigger::PullRequest,
        other => bail!("the desktop release doesn't run on {other:?}"),
    })
}

fn run_plan(ws: &Workspace) -> Result<()> {
    // A git failure must not read as "no tag yet", which would version nightlies 0.1.0.
    let tags = crate::util::git_stdout(&ws.root, &["tag", "--list", "desktop-v*"])
        .context("listing the desktop-v* tags (the checkout needs its tags: fetch-depth 0)")?;
    let inputs = Inputs {
        trigger: trigger()?,
        latest_tag: plan::latest_tag(tags.lines()),
        date: Date::parse(&var("BUILD_DATE")).context("BUILD_DATE (`date -u +%Y%m%d`)")?,
        run_number: var("GITHUB_RUN_NUMBER").parse().context("GITHUB_RUN_NUMBER")?,
        signing: Signing::parse(&var("SIGNED_PLATFORMS"))?,
    };
    let plan = plan::plan(&inputs)?;
    let matrix = serde_json::json!({ "include": plan.legs });
    let commit = var("GITHUB_SHA");
    // What the summary shows; the matrix goes to the outputs only.
    let shown = [
        ("version", plan.version.clone()),
        ("channel", plan.destination.channel.to_owned()),
        ("downloads", plan.destination.downloads.to_owned()),
        ("environment", plan.environment.unwrap_or_default().to_owned()),
        ("dry-run", plan.dry_run().to_string()),
        ("commit", commit),
    ];
    let mut table = String::from("## Desktop release plan\n\n| | |\n| --- | --- |\n");
    for (key, value) in &shown {
        github_output(key, value)?;
        writeln!(table, "| {key} | `{value}` |")?;
        println!("{key}={value}");
    }
    github_output("matrix", &matrix.to_string())?;
    table.push_str("\n| Build | Runner | Version | Bundles |\n| --- | --- | --- | --- |\n");
    for leg in &plan.legs {
        let bundles = if leg.bundles.is_empty() {
            "the OS's"
        } else {
            leg.bundles
        };
        writeln!(table, "| {} | {} | {} | {bundles} |", leg.name, leg.runner, leg.version)?;
    }
    summary(&table)
}

/// The desktop plan's installer budget (§4: under 16 MB per platform; 15 MB until the owner raised it
/// on 2026-11-15 for the engine's on-computer minidump reader), and where it warns (90 %).
const BUDGET: u64 = 16_000_000;
const WARN: u64 = 14_400_000;

/// The installers the budget gates; updater bundles and signatures aren't counted.
fn is_installer(name: &str) -> bool {
    [".dmg", "-setup.exe", ".msi", ".deb", ".rpm"]
        .iter()
        .any(|end| name.ends_with(end))
}

/// Installers reported but not gated: the `AppImage` carries GTK and `WebKitGTK` (about 90 MB), which
/// the `.deb` and `.rpm` take from the system (owner decision 2026-11-15, desktop plan §4).
fn is_reported_only(name: &str) -> bool {
    name.ends_with(".AppImage")
}

fn files_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files_under(&path)?);
        } else {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// How an installer fares against the budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Ok,
    Near,
    Over,
}

impl Verdict {
    fn of(bytes: u64) -> Self {
        match bytes {
            b if b >= BUDGET => Self::Over,
            b if b >= WARN => Self::Near,
            _ => Self::Ok,
        }
    }

    fn words(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Near => "near the budget",
            Self::Over => "over the budget",
        }
    }
}

fn sizes(dir: &Path) -> Result<()> {
    let mut table = String::from("## Installer sizes\n\n| File | Size | Budget 16 MB |\n| --- | ---: | --- |\n");
    let mut over = Vec::new();
    let mut count = 0;
    for path in files_under(dir)? {
        let name = crate::util::file_name(&path);
        if is_reported_only(&name) {
            let bytes = fs::metadata(&path)
                .with_context(|| format!("reading {}", path.display()))?
                .len();
            println!("{name}: {bytes} bytes (reported, not gated)");
            writeln!(table, "| {name} | {} MB | not gated |", bytes / 1_000_000)?;
            continue;
        }
        if !is_installer(&name) {
            continue;
        }
        count += 1;
        let bytes = fs::metadata(&path)
            .with_context(|| format!("reading {}", path.display()))?
            .len();
        let verdict = Verdict::of(bytes);
        match verdict {
            Verdict::Over => {
                println!("::error::{name} is {bytes} bytes, over the 16 MB budget");
                over.push(name.clone());
            }
            Verdict::Near => println!("::warning::{name} is {bytes} bytes, near the 16 MB budget"),
            Verdict::Ok => println!("{name}: {bytes} bytes"),
        }
        writeln!(
            table,
            "| {name} | {}.{:02} MB | {} |",
            bytes / 1_000_000,
            bytes % 1_000_000 / 10_000,
            verdict.words()
        )?;
    }
    summary(&table)?;
    if count == 0 {
        bail!("no installer under {}", dir.display());
    }
    if !over.is_empty() {
        bail!("over the 16 MB budget: {}", over.join(", "));
    }
    Ok(())
}

/// The crates whose SBOMs ship: the app and its engine, as cargo-cyclonedx names their files.
const SBOM_CRATES: [(&str, &str); 2] = [
    ("apps/desktop/src-tauri", "laminapdf-desktop.cdx.json"),
    ("crates/laminapdf-desktop-engine", "laminapdf-desktop-engine.cdx.json"),
];

fn sbom(ws: &Workspace, target: &str, out: &Path) -> Result<()> {
    crate::doctor::require(ws, &crate::doctor::cargo_cyclonedx())?;
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    // One run writes `<crate>.cdx.json` beside every workspace member's manifest, whatever
    // `--manifest-path` names: keep the two that ship and remove the rest.
    ws.cargo(&["cyclonedx", "--format", "json", "--target", target, "--no-build-deps"])?;
    for (dir, name) in SBOM_CRATES {
        let written = ws.root.join(dir).join(name);
        fs::rename(&written, out.join(name)).with_context(|| format!("moving {}", written.display()))?;
    }
    let others = crate::util::git_stdout(
        &ws.root,
        &["ls-files", "--others", "--exclude-standard", "--", "*.cdx.json"],
    )
    .unwrap_or_default();
    for other in others.lines() {
        fs::remove_file(ws.root.join(other)).with_context(|| format!("removing {other}"))?;
    }
    let npm = out.join("laminapdf-desktop-npm.cdx.json");
    let npm = npm.to_str().context("the SBOM path is not UTF-8")?;
    ws.pnpm(&[
        "sbom",
        "--sbom-format",
        "cyclonedx",
        "--prod",
        "--filter",
        "@laminapdf/desktop",
        "--out",
        npm,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installers_should_be_what_people_download() {
        for name in [
            "Lamina PDF_0.1.0_aarch64.dmg",
            "Lamina PDF_0.1.0_x64-setup.exe",
            "Lamina PDF_0.1.0_x64_en-US.msi",
            "lamina-pdf_0.1.0_amd64.deb",
            "lamina-pdf-0.1.0-1.x86_64.rpm",
        ] {
            assert!(is_installer(name), "{name}");
        }
        for name in [
            "Lamina PDF.app.tar.gz",
            "Lamina PDF_0.1.0_x64-setup.exe.sig",
            "SHA256SUMS",
            "x.AppImage.asc",
        ] {
            assert!(!is_installer(name), "{name}");
        }
        let appimage = "Lamina PDF_0.1.0_amd64.AppImage";
        assert!(is_reported_only(appimage) && !is_installer(appimage));
    }

    #[test]
    fn the_budget_should_fail_at_16_mb_and_warn_from_14_4() {
        assert_eq!(Verdict::of(14_399_999), Verdict::Ok);
        assert_eq!(Verdict::of(14_400_000), Verdict::Near);
        assert_eq!(Verdict::of(15_999_999), Verdict::Near);
        assert_eq!(Verdict::of(16_000_000), Verdict::Over);
    }
}
