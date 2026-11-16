//! What a run of the desktop release workflow builds and where it publishes it (ADR 0014 §1):
//! the version, the channel, the GitHub environment and the build matrix, from the event that
//! started the run. A pure function, so the version rules are table-tested.
//!
//! - A `desktop-vX.Y.Z` tag builds `X.Y.Z` for the beta channel in `desktop-release`. Only plain
//!   versions: an MSI's version allows no pre-release, majors and minors up to 255 and patches up
//!   to 65,535 (the `WiX` toolset's limits, which Tauri's bundler checks).
//! - The schedule builds `X.Y.Z-nightly.YYYYMMDD.N` in `desktop-nightly`, `X.Y.Z` the patch after
//!   the latest tag (before any tag, `0.1.0`, the first release), `N` the run number. Windows
//!   builds NSIS only and Linux no `.rpm`: neither an MSI's version nor an rpm's can carry it.
//! - A dispatch builds `0.0.(100 + run)` for the staging channel in `desktop-release`; `only_os`
//!   limits it to one OS, for a trial that mustn't wait on another platform's signing.
//! - A pull request, or a dispatch with `dry_run`, builds every target with no environment and
//!   publishes nothing; a pull request also builds Windows x64 as a tag would, MSI included.
//!
//! A tag publishes only when `SIGNED_PLATFORMS` names both macOS and Windows, as the workflow's
//! does; were a platform left out, a tag would be refused and staging would need
//! `allow_unsigned_platforms`.

use anyhow::{Result, bail};
use serde::Serialize;

use crate::plain_version::{self, Version};

const MAX_MAJOR_MINOR: u64 = 255;
const MAX_PATCH: u64 = 65_535;
/// The first release: the nightlies' version before any tag.
const FIRST_RELEASE: Version = Version(0, 1, 0);
/// Staging versions are `0.0.(STAGING_BASE + run)`, above the updater test's own 0.0.1 and 0.0.2.
const STAGING_BASE: u64 = 100;

/// `text` as a plain `X.Y.Z` within an MSI's limits.
fn parse_plain(text: &str) -> Result<Version> {
    let Some(version) = plain_version::parse(text) else {
        bail!("{text:?} isn't a plain X.Y.Z version (no pre-release or build suffix)");
    };
    if version.0 > MAX_MAJOR_MINOR || version.1 > MAX_MAJOR_MINOR || version.2 > MAX_PATCH {
        bail!("{text}: an MSI version allows majors and minors up to {MAX_MAJOR_MINOR} and patches up to {MAX_PATCH}");
    }
    Ok(version)
}

/// The version a `desktop-v*` tag names.
pub(crate) fn tag_version(tag: &str) -> Result<Version> {
    let Some(text) = tag.strip_prefix("desktop-v") else {
        bail!("{tag:?} isn't a desktop-v<X.Y.Z> tag");
    };
    parse_plain(text)
}

/// The newest of `tags` that names a plain version; other tags are ignored.
pub(crate) fn latest_tag<'a>(tags: impl IntoIterator<Item = &'a str>) -> Option<Version> {
    tags.into_iter().filter_map(|tag| tag_version(tag).ok()).max()
}

/// A UTC date, as `YYYYMMDD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Date(pub u64);

impl Date {
    /// `YYYYMMDD`, eight digits from year 2000 on.
    pub(crate) fn parse(text: &str) -> Result<Self> {
        match plain_version::number(text) {
            Some(n) if text.len() == 8 && n >= 20_000_101 => Ok(Self(n)),
            _ => bail!("{text:?} isn't a YYYYMMDD date"),
        }
    }
}

/// The nightly version after `latest` (the newest tag) on `date`, run `run`.
pub(crate) fn nightly_version(latest: Option<Version>, date: Date, run: u64) -> Result<String> {
    let base = match latest {
        None => FIRST_RELEASE,
        Some(v) if v.2 < MAX_PATCH => Version(v.0, v.1, v.2 + 1),
        Some(v) => bail!("{v} is the last patch an MSI allows: tag the next minor first"),
    };
    if run == 0 {
        bail!("the run number starts at 1");
    }
    Ok(format!("{base}-nightly.{}.{run}", date.0))
}

/// The staging version of run `run`.
pub(crate) fn staging_version(run: u64) -> Result<String> {
    let patch = STAGING_BASE + run;
    if run == 0 || patch > MAX_PATCH {
        bail!("staging run {run} is outside 0.0.101–0.0.{MAX_PATCH}: staging versions have run out");
    }
    Ok(format!("0.0.{patch}"))
}

/// What started the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Trigger {
    Tag(String),
    Schedule,
    Dispatch {
        channel: String,
        dry_run: bool,
        allow_unsigned: bool,
        /// Build and publish this OS's targets only (`only_os`), for a staging trial that
        /// mustn't wait on another platform's signing.
        only: Option<Os>,
    },
    PullRequest,
}

/// Which platforms the workflow signs at the OS level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Signing {
    pub macos: bool,
    pub windows: bool,
}

impl Signing {
    /// `SIGNED_PLATFORMS`: a comma-separated list of `macos` and `windows`.
    pub(crate) fn parse(list: &str) -> Result<Self> {
        let mut signing = Self::default();
        for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            match name {
                "macos" => signing.macos = true,
                "windows" => signing.windows = true,
                other => bail!("SIGNED_PLATFORMS names {other:?}; expected macos or windows"),
            }
        }
        Ok(signing)
    }

    /// Why a release can't publish yet: the platforms not signed at the OS level. `None` once
    /// every platform is.
    fn unsigned_reason(self) -> Option<String> {
        let missing = match (self.macos, self.windows) {
            (true, true) => return None,
            (false, false) => "macOS and Windows aren't",
            (false, true) => "macOS isn't",
            (true, false) => "Windows isn't",
        };
        Some(format!("{missing} signed at the OS level yet (SIGNED_PLATFORMS)"))
    }
}

/// Everything `plan` reads.
#[derive(Debug, Clone)]
pub(crate) struct Inputs {
    pub trigger: Trigger,
    pub latest_tag: Option<Version>,
    pub date: Date,
    pub run_number: u64,
    pub signing: Signing,
}

/// How a build is versioned and bundled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    /// A plain version: NSIS and MSI on Windows.
    Tag,
    /// A nightly pre-release: NSIS only on Windows.
    Nightly,
}

/// A target's OS, as the build steps and the matrix name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Os {
    Macos,
    Windows,
    Linux,
}

impl Os {
    /// The dispatch input `only_os`: `all` (or empty) for every OS, else `macos`, `windows` or
    /// `linux`.
    pub(crate) fn parse_only(text: &str) -> Result<Option<Self>> {
        Ok(match text {
            "" | "all" => None,
            "macos" => Some(Self::Macos),
            "windows" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            other => bail!("only_os is {other:?}; expected all, macos, windows or linux"),
        })
    }
}

/// One target: the hosted runner of its own OS and CPU, the Rust triple, and the update
/// manifest's platform key, `{target}-{arch}` (apps/updates/src/route.ts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Target {
    runner: &'static str,
    triple: &'static str,
    platform: &'static str,
    os: Os,
}

/// The six targets (desktop plan §6 D6). Linux on 22.04, the oldest the app supports, so the
/// packages' glibc and `WebKitGTK` are the floor.
const TARGETS: [Target; 6] = [
    Target {
        runner: "macos-15",
        triple: "aarch64-apple-darwin",
        platform: "darwin-aarch64",
        os: Os::Macos,
    },
    Target {
        runner: "macos-15-intel",
        triple: "x86_64-apple-darwin",
        platform: "darwin-x86_64",
        os: Os::Macos,
    },
    Target {
        runner: "windows-2025",
        triple: "x86_64-pc-windows-msvc",
        platform: "windows-x86_64",
        os: Os::Windows,
    },
    Target {
        runner: "windows-11-arm",
        triple: "aarch64-pc-windows-msvc",
        platform: "windows-aarch64",
        os: Os::Windows,
    },
    Target {
        runner: "ubuntu-22.04",
        triple: "x86_64-unknown-linux-gnu",
        platform: "linux-x86_64",
        os: Os::Linux,
    },
    Target {
        runner: "ubuntu-22.04-arm",
        triple: "aarch64-unknown-linux-gnu",
        platform: "linux-aarch64",
        os: Os::Linux,
    },
];

/// One build of the matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Leg {
    pub name: String,
    pub runner: &'static str,
    pub target: &'static str,
    pub platform: &'static str,
    pub os: Os,
    pub mode: Mode,
    pub version: String,
    /// `--bundles` for the Tauri CLI, or empty for the OS's targets in `tauri.<os>.conf.json`.
    pub bundles: &'static str,
}

impl Target {
    fn leg(self, mode: Mode, version: &str) -> Leg {
        // A nightly's pre-release fits neither an MSI's version nor an rpm's, which allows no `-`.
        let bundles = match (self.os, mode) {
            (Os::Windows, Mode::Nightly) => "nsis",
            (Os::Linux, Mode::Nightly) => "appimage,deb",
            _ => "",
        };
        let suffix = if mode == Mode::Tag { " as a tag" } else { "" };
        Leg {
            name: format!("{}{suffix}", self.platform),
            runner: self.runner,
            target: self.triple,
            platform: self.platform,
            os: self.os,
            mode,
            version: version.to_owned(),
            bundles,
        }
    }
}

/// A channel and the address its bucket serves downloads from (ADR 0014 §1). The bucket itself is
/// the manifest tool's to know (apps/updates/src/channels.ts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Destination {
    pub channel: &'static str,
    pub downloads: &'static str,
}

const BETA: Destination = Destination {
    channel: "beta",
    downloads: "https://downloads.laminapdf.com",
};
const NIGHTLY: Destination = Destination {
    channel: "nightly",
    downloads: "https://nightly-downloads.laminapdf.com",
};
const STAGING: Destination = Destination {
    channel: "staging",
    downloads: "https://staging-downloads.laminapdf.com",
};

/// The run's plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    /// The version published; for a dry run, the version its nightly-mode legs build.
    pub version: String,
    pub destination: Destination,
    /// The GitHub environment of the build and publish jobs; `None` for a dry run.
    pub environment: Option<&'static str>,
    pub legs: Vec<Leg>,
}

impl Plan {
    pub(crate) fn dry_run(&self) -> bool {
        self.environment.is_none()
    }
}

fn every_target(mode: Mode, version: &str) -> Vec<Leg> {
    TARGETS.iter().map(|t| t.leg(mode, version)).collect()
}

/// The plan for `inputs`.
pub(crate) fn plan(inputs: &Inputs) -> Result<Plan> {
    match &inputs.trigger {
        Trigger::Tag(tag) => {
            let version = tag_version(tag)?.to_string();
            if let Some(reason) = inputs.signing.unsigned_reason() {
                bail!(
                    "{reason}: until every platform is, only the nightly channel publishes, so {tag} can't be released"
                );
            }
            Ok(Plan {
                legs: every_target(Mode::Tag, &version),
                version,
                destination: BETA,
                environment: Some("desktop-release"),
            })
        }
        Trigger::Schedule => {
            let version = nightly_version(inputs.latest_tag, inputs.date, inputs.run_number)?;
            Ok(Plan {
                legs: every_target(Mode::Nightly, &version),
                version,
                destination: NIGHTLY,
                environment: Some("desktop-nightly"),
            })
        }
        Trigger::Dispatch {
            channel,
            dry_run,
            allow_unsigned,
            only,
        } => {
            if channel != "staging" {
                bail!(
                    "a dispatch publishes to staging only, not {channel:?}: beta and stable come from desktop-v* tags, nightly from the schedule"
                );
            }
            let version = staging_version(inputs.run_number)?;
            if !dry_run
                && !allow_unsigned
                && let Some(reason) = inputs.signing.unsigned_reason()
            {
                bail!("{reason}: set allow_unsigned_platforms to publish an unsigned staging build");
            }
            let mut legs = every_target(Mode::Tag, &version);
            if let Some(os) = only {
                legs.retain(|leg| leg.os == *os);
            }
            Ok(Plan {
                legs,
                version,
                destination: STAGING,
                environment: (!dry_run).then_some("desktop-release"),
            })
        }
        Trigger::PullRequest => {
            let version = nightly_version(inputs.latest_tag, inputs.date, inputs.run_number)?;
            let mut legs = every_target(Mode::Nightly, &version);
            let tag_mode = staging_version(inputs.run_number)?;
            legs.extend(
                TARGETS
                    .iter()
                    .filter(|t| t.platform == "windows-x86_64")
                    .map(|t| t.leg(Mode::Tag, &tag_mode)),
            );
            Ok(Plan {
                version,
                destination: NIGHTLY,
                environment: None,
                legs,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    const DATE: Date = Date(20_261_028);

    fn inputs(trigger: Trigger) -> Inputs {
        Inputs {
            trigger,
            latest_tag: Some(Version(0, 1, 0)),
            date: DATE,
            run_number: 42,
            signing: Signing::default(),
        }
    }

    fn signed(trigger: Trigger) -> Inputs {
        Inputs {
            signing: Signing::parse("macos, windows").unwrap(),
            ..inputs(trigger)
        }
    }

    fn dispatch(channel: &str, dry_run: bool, allow_unsigned: bool) -> Trigger {
        Trigger::Dispatch {
            channel: channel.to_owned(),
            dry_run,
            allow_unsigned,
            only: None,
        }
    }

    /// Compares two pre-release identifiers as semver does: numbers numerically and below words.
    fn identifier_order(x: &str, y: &str) -> Ordering {
        match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        }
    }

    /// Whether `a` sorts before `b` as the updater compares versions: a pre-release below its release.
    fn semver_lt(a: &str, b: &str) -> bool {
        fn split(s: &str) -> (Version, Option<Vec<&str>>) {
            let (core, pre) = s.split_once('-').map_or((s, None), |(c, p)| (c, Some(p)));
            (plain_version::parse(core).unwrap(), pre.map(|p| p.split('.').collect()))
        }
        let ((ca, pa), (cb, pb)) = (split(a), split(b));
        if ca != cb {
            return ca < cb;
        }
        match (pa, pb) {
            (Some(_), None) => true,
            (None, _) => false,
            (Some(pa), Some(pb)) => pa
                .iter()
                .zip(&pb)
                .map(|(x, y)| identifier_order(x, y))
                .find(|o| o.is_ne())
                .map_or(pa.len() < pb.len(), Ordering::is_lt),
        }
    }

    fn no_leading_zero(version: &str) -> bool {
        version
            .split(['.', '-'])
            .filter(|id| id.bytes().all(|b| b.is_ascii_digit()))
            .all(|id| id == "0" || !id.starts_with('0'))
    }

    /// `^\d+\.\d+\.\d+-nightly\.[1-9]\d{7}\.[1-9]\d*$`, the todo's nightly pattern.
    fn is_nightly(version: &str) -> bool {
        let Some((core, pre)) = version.split_once("-nightly.") else {
            return false;
        };
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        let core_ok = core.split('.').count() == 3 && core.split('.').all(digits);
        pre.split_once('.').is_some_and(|(date, run)| {
            core_ok && digits(date) && date.len() == 8 && !date.starts_with('0') && digits(run) && !run.starts_with('0')
        })
    }

    /// `^0\.0\.(10[1-9]|1[1-9]\d|[2-9]\d{2}|[1-9]\d{3,4})$`, the todo's staging pattern.
    fn is_staging(version: &str) -> bool {
        version
            .strip_prefix("0.0.")
            .and_then(plain_version::number)
            .is_some_and(|patch| (101..=99_999).contains(&patch))
    }

    #[test]
    fn a_tag_should_name_a_plain_version_within_the_msi_limits() {
        let table = [
            ("desktop-v0.1.0", Some(Version(0, 1, 0))),
            ("desktop-v255.255.65535", Some(Version(255, 255, 65_535))),
            ("desktop-v0.2.0-rc.1", None),
            ("desktop-v0.2.0+build.5", None),
            ("desktop-v256.0.0", None),
            ("desktop-v0.256.0", None),
            ("desktop-v0.1.65536", None),
            ("desktop-v01.1.0", None),
            ("desktop-v0.1", None),
            ("v0.1.0", None),
        ];
        for (tag, expected) in table {
            assert_eq!(tag_version(tag).ok(), expected, "{tag}");
        }
    }

    #[test]
    fn the_latest_tag_should_be_the_highest_plain_version() {
        let tags = [
            "desktop-v0.1.0",
            "desktop-v0.10.0",
            "desktop-v0.9.3",
            "desktop-v1.0.0-rc.1",
            "junk",
        ];
        assert_eq!(latest_tag(tags), Some(Version(0, 10, 0)));
        assert_eq!(latest_tag(["junk"]), None);
    }

    #[test]
    fn a_nightly_should_be_the_next_patch_dated_and_numbered() {
        assert_eq!(
            nightly_version(Some(Version(0, 1, 0)), DATE, 42).unwrap(),
            "0.1.1-nightly.20261028.42"
        );
        assert_eq!(nightly_version(None, DATE, 42).unwrap(), "0.1.0-nightly.20261028.42");
        assert!(nightly_version(Some(Version(0, 1, 65_535)), DATE, 1).is_err());
        assert!(nightly_version(None, DATE, 0).is_err());
    }

    #[test]
    fn a_nightly_should_sort_after_the_last_release_and_before_the_next() {
        let latest = Some(Version(0, 1, 0));
        let after_tag = nightly_version(latest, DATE, 42).unwrap();
        assert!(semver_lt("0.1.0", &after_tag));
        assert!(semver_lt(&after_tag, "0.1.1"));
        let before_tag = nightly_version(None, DATE, 42).unwrap();
        assert!(semver_lt("0.0.65535", &before_tag));
        assert!(semver_lt(&before_tag, "0.1.0"));
        // A later run, or a later day, sorts after.
        assert!(semver_lt(&after_tag, &nightly_version(latest, DATE, 43).unwrap()));
        assert!(semver_lt(
            &after_tag,
            &nightly_version(latest, Date(20_261_029), 1).unwrap()
        ));
    }

    #[test]
    fn every_version_should_match_its_pattern_with_no_leading_zero() {
        for run in [1, 9, 10, 99, 100, 999, 65_435] {
            let n = nightly_version(Some(Version(0, 1, 0)), DATE, run).unwrap();
            assert!(is_nightly(&n) && no_leading_zero(&n), "{n}");
            let s = staging_version(run).unwrap();
            assert!(is_staging(&s) && no_leading_zero(&s), "{s}");
        }
        assert!(!is_nightly("0.1.1-nightly.02261028.1") && !is_staging("0.0.100"));
    }

    #[test]
    fn staging_should_be_100_plus_the_run_up_to_the_msi_limit() {
        assert_eq!(staging_version(7).unwrap(), "0.0.107");
        assert_eq!(staging_version(65_435).unwrap(), "0.0.65535");
        assert!(staging_version(65_436).is_err());
        assert!(staging_version(0).is_err());
    }

    #[test]
    fn dates_should_be_eight_digits() {
        assert_eq!(Date::parse("20261028").unwrap(), DATE);
        for bad in ["2026-10-28", "0261028", "1999123", "019991231", ""] {
            assert!(Date::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_schedule_should_publish_nightly_in_the_nightly_environment() {
        let plan = plan(&inputs(Trigger::Schedule)).unwrap();
        assert_eq!(plan.environment, Some("desktop-nightly"));
        assert_eq!(plan.destination, NIGHTLY);
        assert_eq!(plan.legs.len(), 6);
        assert!(
            plan.legs
                .iter()
                .all(|l| l.mode == Mode::Nightly && l.version == plan.version)
        );
    }

    #[test]
    fn a_nightly_should_build_no_msi_and_no_rpm() {
        let plan = plan(&inputs(Trigger::Schedule)).unwrap();
        for leg in &plan.legs {
            let expected = match leg.os {
                Os::Windows => "nsis",
                Os::Linux => "appimage,deb",
                Os::Macos => "",
            };
            assert_eq!(leg.bundles, expected, "{}", leg.name);
        }
    }

    #[test]
    fn a_tag_should_wait_for_macos_and_windows_signing() {
        let tag = || Trigger::Tag("desktop-v0.1.0".to_owned());
        assert!(plan(&inputs(tag())).is_err());
        let half = Inputs {
            signing: Signing {
                macos: true,
                windows: false,
            },
            ..inputs(tag())
        };
        assert!(plan(&half).is_err());
        let plan = plan(&signed(tag())).unwrap();
        assert_eq!(
            (plan.version.as_str(), plan.environment),
            ("0.1.0", Some("desktop-release"))
        );
        assert_eq!(plan.destination, BETA);
        assert!(plan.legs.iter().all(|l| l.mode == Mode::Tag && l.bundles.is_empty()));
    }

    #[test]
    fn a_pre_release_tag_should_fail_the_plan() {
        assert!(plan(&signed(Trigger::Tag("desktop-v0.2.0-rc.1".to_owned()))).is_err());
    }

    #[test]
    fn a_dispatch_should_publish_to_staging_only() {
        for channel in ["beta", "stable", "nightly", ""] {
            assert!(plan(&inputs(dispatch(channel, false, true))).is_err(), "{channel}");
        }
        let plan = plan(&inputs(dispatch("staging", false, true))).unwrap();
        assert_eq!(
            (plan.version.as_str(), plan.environment),
            ("0.0.142", Some("desktop-release"))
        );
        assert_eq!(plan.destination, STAGING);
    }

    #[test]
    fn a_staging_dispatch_for_one_os_should_build_only_its_targets() {
        let trigger = Trigger::Dispatch {
            channel: "staging".to_owned(),
            dry_run: false,
            allow_unsigned: false,
            only: Os::parse_only("windows").unwrap(),
        };
        let plan = plan(&signed(trigger)).unwrap();
        let platforms: Vec<&str> = plan.legs.iter().map(|leg| leg.platform).collect();
        assert_eq!(platforms, ["windows-x86_64", "windows-aarch64"]);
    }

    #[test]
    fn only_os_should_name_all_or_an_os() {
        assert_eq!(
            ["", "all", "linux"].map(|text| Os::parse_only(text).unwrap()),
            [None, None, Some(Os::Linux)]
        );
        assert!(Os::parse_only("android").is_err());
    }

    #[test]
    fn unsigned_staging_should_need_the_explicit_input() {
        assert!(plan(&inputs(dispatch("staging", false, false))).is_err());
        assert!(plan(&signed(dispatch("staging", false, false))).is_ok());
        assert!(plan(&inputs(dispatch("staging", true, false))).unwrap().dry_run());
    }

    #[test]
    fn a_pull_request_should_dry_run_every_target_and_windows_as_a_tag() {
        let plan = plan(&inputs(Trigger::PullRequest)).unwrap();
        assert!(plan.dry_run());
        assert_eq!(plan.legs.len(), 7);
        let tag_legs: Vec<&Leg> = plan.legs.iter().filter(|l| l.mode == Mode::Tag).collect();
        assert_eq!(tag_legs.len(), 1);
        assert_eq!((tag_legs[0].platform, tag_legs[0].bundles), ("windows-x86_64", ""));
        assert!(parse_plain(&tag_legs[0].version).is_ok());
    }

    #[test]
    fn signed_platforms_should_name_macos_or_windows() {
        assert_eq!(Signing::parse("").unwrap(), Signing::default());
        assert!(Signing::parse("linux").is_err());
    }

    #[test]
    fn the_refusal_should_name_only_the_platforms_still_unsigned() {
        let reason = |list| Signing::parse(list).unwrap().unsigned_reason();
        assert_eq!(
            [reason(""), reason("macos"), reason("macos,windows")],
            [
                Some("macOS and Windows aren't signed at the OS level yet (SIGNED_PLATFORMS)".to_owned()),
                Some("Windows isn't signed at the OS level yet (SIGNED_PLATFORMS)".to_owned()),
                None,
            ]
        );
    }

    /// The workflow's own `SIGNED_PLATFORMS` parses and leaves no platform unsigned: tags publish.
    #[test]
    fn the_workflow_should_sign_both_platforms() {
        let workflow = crate::workflows::repository_file(".github/workflows/desktop-release.yml").unwrap();
        let line = workflow
            .lines()
            .find(|l| l.trim_start().starts_with("SIGNED_PLATFORMS:"))
            .unwrap();
        let list = line.split_once(':').unwrap().1.trim().trim_matches('"');
        assert_eq!(Signing::parse(list).unwrap().unsigned_reason(), None, "{line}");
    }

    /// The update Worker's lists are the source: apps/updates/src/channels.ts names the channels,
    /// and route.ts the targets and arches a platform key is made of.
    #[test]
    fn channels_and_platform_keys_should_be_the_update_workers() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/updates/src");
        let channels = std::fs::read_to_string(root.join("channels.ts")).unwrap();
        for destination in [BETA, NIGHTLY, STAGING] {
            assert!(
                channels.contains(&format!("\"{}\"", destination.channel)),
                "{}",
                destination.channel
            );
        }
        let route = std::fs::read_to_string(root.join("route.ts")).unwrap();
        for target in TARGETS {
            let (os, arch) = target.platform.split_once('-').unwrap();
            assert!(
                route.contains(&format!("\"{os}\"")) && route.contains(&format!("\"{arch}\"")),
                "{}",
                target.platform
            );
        }
    }
}
