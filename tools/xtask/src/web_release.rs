//! The web reader's own releases (todo C7e): versioned apart from the packages, cut by the web
//! release train (`.github/workflows/web-release-train.yml`) once a day after the full nightly
//! passes, but only when the reader changed since its last release.
//!
//! - `cargo xtask web next-release [--commit <rev>] [--notes <file>]` decides whether `<rev>`
//!   (default `HEAD`) is a new release: what shipped changed since the newest `web-v*` tag, and
//!   the version it gets. It prints `ready`, `tag` and `previous`, appends them to
//!   `$GITHUB_OUTPUT`, and writes the release notes (one line per commit that changed what
//!   shipped) to `<file>`.
//! - `cargo xtask web check-tag <tag>` fails unless `<tag>` is `web-v<version>` with the major
//!   and minor of `apps/web/package.json`'s version and a patch at least its patch: the deploy
//!   runs it before building, so a tag can't ship a version `package.json` didn't allow.
//!
//! The patch number goes up by itself. A new minor or major is the owner's: raise
//! `apps/web/package.json`'s version, and the next release takes it as written.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::tracked::git;

const PACKAGE: &str = "apps/web/package.json";
const TAG_PREFIX: &str = "web-v";

/// A release version: `MAJOR.MINOR.PATCH`, no pre-release (the reader releases only these).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u64, u64, u64);

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

fn parse(text: &str) -> Option<Version> {
    let mut parts = text.split('.');
    let mut number = || -> Option<u64> {
        let part = parts.next()?;
        // No leading zeros, no signs: semver's numeric identifiers.
        (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) && (part == "0" || !part.starts_with('0')))
            .then(|| part.parse().ok())
            .flatten()
    };
    let version = Version(number()?, number()?, number()?);
    parts.next().is_none().then_some(version)
}

/// The version after `latest` that `base` (`package.json`'s) allows: `base` itself when it is newer
/// (the first release, or the owner raised it), else `latest` with its patch one higher, as long as
/// `base` is on `latest`'s major and minor.
fn next_version(base: Version, latest: Option<Version>) -> Result<Version> {
    let Some(latest) = latest else {
        return Ok(base);
    };
    if base > latest {
        return Ok(base);
    }
    if (base.0, base.1) != (latest.0, latest.1) {
        bail!("{PACKAGE}'s version {base} is behind the released {TAG_PREFIX}{latest}: raise it to {latest} or later");
    }
    Ok(Version(latest.0, latest.1, latest.2 + 1))
}

/// Whether `path` can change what visitors to the reader get: the reader's inputs
/// ([`crate::changes::touches_web`]) except the deploy machinery (`tools/xtask/` and the
/// workflows), which the preview and staging deploys count but a release needn't, and Markdown
/// (READMEs, changelogs), which the reader doesn't bundle.
fn ships(path: &str) -> bool {
    crate::changes::touches_web(&[path])
        && !path.starts_with("tools/")
        && !path.starts_with(".github/")
        && !Path::new(path)
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("md"))
}

fn base_version(root: &Path) -> Result<Version> {
    let path = root.join(PACKAGE);
    let package: Value =
        serde_json::from_str(&fs::read_to_string(&path).with_context(|| format!("reading {PACKAGE}"))?)
            .with_context(|| format!("{PACKAGE} is not JSON"))?;
    let version = package
        .get("version")
        .and_then(Value::as_str)
        .with_context(|| format!("{PACKAGE} has no version"))?;
    parse(version).with_context(|| format!("{PACKAGE}'s version {version:?} is not MAJOR.MINOR.PATCH"))
}

/// The newest `web-v*` tag and its version; tags that aren't a release version are left out.
fn latest_tag(root: &Path) -> Result<Option<(String, Version)>> {
    let tags = git(root, &["tag", "--list", &format!("{TAG_PREFIX}*")])?;
    Ok(tags
        .lines()
        .filter_map(|tag| Some((tag.to_owned(), parse(tag.strip_prefix(TAG_PREFIX)?)?)))
        .max_by_key(|(_, version)| *version))
}

/// The subject of each first-parent commit in `range` whose changes include a path that ships.
fn notes(root: &Path, range: &str) -> Result<Vec<String>> {
    let log = git(root, &["log", "--first-parent", "--format=%H%x00%s", range])?;
    let mut lines = Vec::new();
    for entry in log.lines() {
        let Some((sha, subject)) = entry.split_once('\0') else {
            continue;
        };
        let files = git(
            root,
            &[
                "diff-tree",
                "--no-commit-id",
                "--name-only",
                "-r",
                "-m",
                "--first-parent",
                sha,
            ],
        )?;
        if files.lines().any(ships) {
            lines.push(format!("- {subject}"));
        }
    }
    Ok(lines)
}

/// `web next-release`'s arguments: the commit (default `HEAD`) and the notes file.
pub(crate) fn parse_next_release(args: &[String]) -> Result<(String, Option<std::path::PathBuf>)> {
    let mut commit = "HEAD".to_owned();
    let mut notes = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let value = || {
            args.clone()
                .next()
                .cloned()
                .with_context(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--commit" => commit = value()?,
            "--notes" => notes = Some(value()?.into()),
            other => bail!("unknown argument {other:?} (web next-release [--commit <rev>] [--notes <file>])"),
        }
        args.next();
    }
    Ok((commit, notes))
}

/// `cargo xtask web next-release`.
pub(crate) fn next_release(root: &Path, commit: &str, notes_file: Option<&Path>) -> Result<()> {
    let commit = git(root, &["rev-parse", "--verify", &format!("{commit}^{{commit}}")])?;
    let commit = commit.trim();
    let base = base_version(root)?;
    let latest = latest_tag(root)?;
    let (ready, lines) = match &latest {
        None => (true, vec!["- The web reader's first release.".to_owned()]),
        Some((tag, _)) => {
            let range = format!("{tag}..{commit}");
            let lines = notes(root, &range)?;
            (!lines.is_empty(), lines)
        }
    };
    let version = next_version(base, latest.as_ref().map(|(_, v)| *v))?;
    let tag = format!("{TAG_PREFIX}{version}");
    let previous = latest.map(|(tag, _)| tag).unwrap_or_default();
    match (ready, previous.is_empty()) {
        (false, _) => println!("No change to what the reader ships since {previous}: nothing to release."),
        (true, true) => println!("{commit} is the reader's first release, {tag}."),
        (true, false) => println!("{commit} is {tag}: {} change(s) since {previous}.", lines.len()),
    }
    if ready && let Some(path) = notes_file {
        fs::write(path, format!("{}\n", lines.join("\n"))).with_context(|| format!("writing {}", path.display()))?;
    }
    crate::util::github_output("ready", &ready.to_string())?;
    crate::util::github_output("tag", &tag)?;
    crate::util::github_output("previous", &previous)
}

/// `cargo xtask web check-tag <tag>`.
pub(crate) fn check_tag(root: &Path, tag: &str) -> Result<()> {
    let base = base_version(root)?;
    let Some(version) = tag.strip_prefix(TAG_PREFIX).and_then(parse) else {
        bail!("{tag:?} is not a reader release tag ({TAG_PREFIX}MAJOR.MINOR.PATCH)");
    };
    if (version.0, version.1) != (base.0, base.1) || version < base {
        bail!(
            "{tag} doesn't match {PACKAGE}'s version {base}: a release keeps its major and minor, with a patch of at least {}",
            base.2
        );
    }
    println!("{tag} matches {PACKAGE}'s version {base}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Version, next_version, parse, ships};

    #[test]
    fn parse_should_accept_release_versions_only() {
        assert_eq!(parse("0.1.0"), Some(Version(0, 1, 0)));
        assert_eq!(parse("10.20.30"), Some(Version(10, 20, 30)));
        for refused in ["0.1", "0.1.0.1", "0.1.0-alpha.0", "01.1.0", "v0.1.0", "0.1.x", ""] {
            assert_eq!(parse(refused), None, "{refused}");
        }
    }

    #[test]
    fn next_version_should_take_package_json_first_then_bump_the_patch() {
        let v = |a, b, c| Version(a, b, c);
        assert_eq!(next_version(v(0, 1, 0), None).unwrap(), v(0, 1, 0));
        assert_eq!(next_version(v(0, 1, 0), Some(v(0, 1, 0))).unwrap(), v(0, 1, 1));
        assert_eq!(next_version(v(0, 1, 0), Some(v(0, 1, 7))).unwrap(), v(0, 1, 8));
        // The owner raised it: taken as written.
        assert_eq!(next_version(v(0, 2, 0), Some(v(0, 1, 7))).unwrap(), v(0, 2, 0));
        assert_eq!(next_version(v(0, 1, 9), Some(v(0, 1, 7))).unwrap(), v(0, 1, 9));
        // Behind a release on another line: refused, never a silent downgrade.
        assert!(next_version(v(0, 1, 0), Some(v(0, 2, 3))).is_err());
    }

    #[test]
    fn ships_should_count_the_reader_and_what_it_bundles_but_not_the_deploy_machinery() {
        for path in [
            "apps/web/src/app.tsx",
            "packages/viewer/src/client.ts",
            "crates/laminapdf/src/lib.rs",
            "pnpm-lock.yaml",
        ] {
            assert!(ships(path), "{path}");
        }
        for path in [
            "tools/xtask/src/web.rs",
            ".github/workflows/web-deploy.yml",
            "apps/site/src/pages/index.astro",
            "docs/plans/web-app.md",
            "apps/web/README.md",
        ] {
            assert!(!ships(path), "{path}");
        }
    }

    const TRAIN: &str = include_str!("../../release/web-release-train.yml");

    /// The owner installs the train from tools/release: never merge a partial installation.
    #[test]
    fn installed_web_release_train_should_match_the_reviewed_file() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/web-release-train.yml");
        assert!(
            std::fs::read_to_string(&path).is_ok_and(|installed| installed == TRAIN),
            "the owner must install tools/release/web-release-train.yml as .github/workflows/web-release-train.yml before merge"
        );
    }

    /// Only a full successful nightly on main releases the reader, the kill switch stops it, and
    /// only the release bot's token (whose push starts web-deploy.yml) creates the tag.
    #[test]
    fn web_release_train_should_tag_only_a_full_nightlys_commit_through_the_bot() {
        use crate::workflows::lines_of;
        let trigger = lines_of(TRAIN, "on", "workflow_run").join("\n");
        for condition in ["workflows: [Nightly]", "types: [completed]", "branches: [main]"] {
            assert!(trigger.contains(condition), "missing trigger {condition}");
        }
        let job = lines_of(TRAIN, "jobs", "tag").join("\n");
        for part in [
            "vars.WEB_RELEASE_TRAIN != 'off'",
            "github.event.workflow_run.conclusion == 'success'",
            "github.event.workflow_run.head_branch == 'main'",
            "github.event.workflow_run.head_repository.full_name == github.repository",
            "ref: ${{ steps.nightly.outputs.sha }}",
            "cargo xtask web next-release --commit \"$SHA\" --notes \"$RUNNER_TEMP/notes.md\"",
            "GH_TOKEN: ${{ steps.bot.outputs.token }}",
            "repos/$GITHUB_REPOSITORY/git/tags",
            "--raw-field ref=\"refs/tags/$TAG\"",
        ] {
            assert!(job.contains(part), "the web release train lacks {part}");
        }
        assert!(
            !TRAIN.lines().any(|line| line.trim() == "contents: write"),
            "the train's own token stays read-only"
        );
        assert_eq!(job.matches("if: steps.next.outputs.ready == 'true'").count(), 2);
    }
}
