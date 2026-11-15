//! What a run of the desktop release workflow builds and where it publishes it (todo D6b,
//! ADR 0014 §1): the version, the channel, the GitHub environment and the build matrix, from the
//! event that started the run. A pure function, so the version rules are table-tested.
//!
//! - A `desktop-vX.Y.Z` tag builds `X.Y.Z` for the beta channel in `desktop-release`. Only plain
//!   versions: the `WiX` toolset's MSI version allows no pre-release, majors and minors up to 255 and patches up
//!   to 65,535.
//! - The schedule builds `X.Y.Z-nightly.YYYYMMDD.N` in `desktop-nightly`, `X.Y.Z` the patch after
//!   the latest tag (before any tag, `0.1.0`, the first release), `N` the run number. Windows
//!   builds NSIS only: MSI can't carry the pre-release.
//! - A dispatch builds `0.0.(100 + run)` for the staging channel in `desktop-release`.
//! - A pull request, or a dispatch with `dry_run`, builds every target with no environment and
//!   publishes nothing; a pull request also builds Windows x64 as a tag would, MSI included.
//!
//! Until macOS and Windows are signed (D6c, D6d), a tag is refused and staging needs
//! `allow_unsigned_platforms`.

use std::fmt;

use anyhow::{Result, bail};
use serde::Serialize;

/// A plain `major.minor.patch` version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// the `WiX` toolset's limits for an MSI's `major.minor.build` (and Tauri's bundler checks the same).
const MAX_MAJOR_MINOR: u32 = 255;
const MAX_PATCH: u32 = 65_535;
/// The first release (D7c): the nightlies' version before any tag.
const FIRST_RELEASE: Version = Version {
    major: 0,
    minor: 1,
    patch: 0,
};
/// Staging versions are `0.0.(STAGING_BASE + run)`, above D5b's own test versions 0.0.1 and 0.0.2.
const STAGING_BASE: u64 = 100;

/// One numeric identifier: digits only, no leading zero.
fn number(text: &str) -> Option<u32> {
    let plain = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) && (text == "0" || !text.starts_with('0'));
    plain.then(|| text.parse().ok()).flatten()
}

impl Version {
    /// `X.Y.Z` with no pre-release or build, within the `WiX` toolset's limits.
    pub(crate) fn parse_plain(text: &str) -> Result<Self> {
        let parts: Vec<&str> = text.split('.').collect();
        let [major, minor, patch] = parts.as_slice() else {
            bail!("{text:?} isn't a plain X.Y.Z version (no pre-release or build suffix)");
        };
        let (Some(major), Some(minor), Some(patch)) = (number(major), number(minor), number(patch)) else {
            bail!("{text:?} isn't a plain X.Y.Z version (no pre-release or build suffix)");
        };
        if major > MAX_MAJOR_MINOR || minor > MAX_MAJOR_MINOR || patch > MAX_PATCH {
            bail!(
                "{text}: an MSI version allows majors and minors up to {MAX_MAJOR_MINOR} and patches up to {MAX_PATCH}"
            );
        }
        Ok(Self { major, minor, patch })
    }
}

/// The version a `desktop-v*` tag names.
pub(crate) fn tag_version(tag: &str) -> Result<Version> {
    let Some(text) = tag.strip_prefix("desktop-v") else {
        bail!("{tag:?} isn't a desktop-v<X.Y.Z> tag");
    };
    Version::parse_plain(text)
}

/// The newest of `tags` that names a plain version; other tags are ignored.
pub(crate) fn latest_tag<'a>(tags: impl IntoIterator<Item = &'a str>) -> Option<Version> {
    tags.into_iter().filter_map(|tag| tag_version(tag).ok()).max()
}

/// A UTC date, as `YYYYMMDD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Date(pub u32);

impl Date {
    /// `YYYYMMDD`, eight digits from year 2000 on.
    pub(crate) fn parse(text: &str) -> Result<Self> {
        match number(text) {
            Some(n) if text.len() == 8 && n >= 20_000_101 => Ok(Self(n)),
            _ => bail!("{text:?} isn't a YYYYMMDD date"),
        }
    }
}

/// The nightly version after `latest` (the newest tag) on `date`, run `run`.
pub(crate) fn nightly_version(latest: Option<Version>, date: Date, run: u64) -> Result<String> {
    let base = match latest {
        None => FIRST_RELEASE,
        Some(v) if v.patch < MAX_PATCH => Version {
            patch: v.patch + 1,
            ..v
        },
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
    if run == 0 || patch > u64::from(MAX_PATCH) {
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
    },
    PullRequest,
}

/// Which platforms the workflow can sign at the OS level (D6c: macOS, D6d: Windows).
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

    fn all(self) -> bool {
        self.macos && self.windows
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

/// One build of the matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Leg {
    pub name: String,
    pub runner: &'static str,
    pub target: &'static str,
    /// The update manifest's key, `{target}-{arch}` (apps/updates/src/route.ts).
    pub platform: &'static str,
    pub os: &'static str,
    pub mode: Mode,
    pub version: String,
    /// `--bundles` for the Tauri CLI, or empty for the OS's targets in tauri.<os>.conf.json.
    pub bundles: &'static str,
}

/// The six targets (desktop plan §6 D6), each on a hosted runner of its own OS and CPU. Linux on
/// 22.04, the oldest the app supports, so the packages' glibc and `WebKitGTK` are the floor.
const TARGETS: [(&str, &str, &str, &str); 6] = [
    ("macos-15", "aarch64-apple-darwin", "darwin-aarch64", "macos"),
    ("macos-15-intel", "x86_64-apple-darwin", "darwin-x86_64", "macos"),
    ("windows-2025", "x86_64-pc-windows-msvc", "windows-x86_64", "windows"),
    (
        "windows-11-arm",
        "aarch64-pc-windows-msvc",
        "windows-aarch64",
        "windows",
    ),
    ("ubuntu-22.04", "x86_64-unknown-linux-gnu", "linux-x86_64", "linux"),
    (
        "ubuntu-22.04-arm",
        "aarch64-unknown-linux-gnu",
        "linux-aarch64",
        "linux",
    ),
];

fn leg(
    &(runner, target, platform, os): &(&'static str, &'static str, &'static str, &'static str),
    mode: Mode,
    version: &str,
) -> Leg {
    let bundles = if os == "windows" && mode == Mode::Nightly {
        "nsis"
    } else {
        ""
    };
    let suffix = if mode == Mode::Tag { " as a tag" } else { "" };
    Leg {
        name: format!("{platform}{suffix}"),
        runner,
        target,
        platform,
        os,
        mode,
        version: version.to_owned(),
        bundles,
    }
}

/// Where a channel publishes (ADR 0014 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Destination {
    pub channel: &'static str,
    pub bucket: &'static str,
    pub downloads: &'static str,
}

const BETA: Destination = Destination {
    channel: "beta",
    bucket: "lamina-pdf-releases",
    downloads: "https://downloads.laminapdf.com",
};
const NIGHTLY: Destination = Destination {
    channel: "nightly",
    bucket: "lamina-pdf-nightly",
    downloads: "https://nightly-downloads.laminapdf.com",
};
const STAGING: Destination = Destination {
    channel: "staging",
    bucket: "lamina-pdf-staging",
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

const UNSIGNED: &str = "macOS and Windows aren't signed yet (D6c, D6d)";

/// The plan for `inputs`.
pub(crate) fn plan(inputs: &Inputs) -> Result<Plan> {
    let all = |mode: Mode, version: &str| TARGETS.iter().map(|t| leg(t, mode, version)).collect::<Vec<_>>();
    match &inputs.trigger {
        Trigger::Tag(tag) => {
            let version = tag_version(tag)?.to_string();
            if !inputs.signing.all() {
                bail!("{UNSIGNED}: until they are, only the nightly channel publishes, so {tag} can't be released");
            }
            Ok(Plan {
                legs: all(Mode::Tag, &version),
                version,
                destination: BETA,
                environment: Some("desktop-release"),
            })
        }
        Trigger::Schedule => {
            let version = nightly_version(inputs.latest_tag, inputs.date, inputs.run_number)?;
            Ok(Plan {
                legs: all(Mode::Nightly, &version),
                version,
                destination: NIGHTLY,
                environment: Some("desktop-nightly"),
            })
        }
        Trigger::Dispatch {
            channel,
            dry_run,
            allow_unsigned,
        } => {
            if channel != "staging" {
                bail!(
                    "a dispatch publishes to staging only, not {channel:?}: beta and stable come from desktop-v* tags, nightly from the schedule"
                );
            }
            let version = staging_version(inputs.run_number)?;
            if !dry_run && !inputs.signing.all() && !allow_unsigned {
                bail!("{UNSIGNED}: set allow_unsigned_platforms to publish an unsigned staging build");
            }
            Ok(Plan {
                legs: all(Mode::Tag, &version),
                version,
                destination: STAGING,
                environment: (!dry_run).then_some("desktop-release"),
            })
        }
        Trigger::PullRequest => {
            let version = nightly_version(inputs.latest_tag, inputs.date, inputs.run_number)?;
            let mut legs = all(Mode::Nightly, &version);
            let tag_mode = staging_version(inputs.run_number)?;
            legs.extend(
                TARGETS
                    .iter()
                    .filter(|t| t.2 == "windows-x86_64")
                    .map(|t| leg(t, Mode::Tag, &tag_mode)),
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
    use super::*;

    const DATE: Date = Date(20_261_028);

    fn v(major: u32, minor: u32, patch: u32) -> Version {
        Version { major, minor, patch }
    }

    fn inputs(trigger: Trigger) -> Inputs {
        Inputs {
            trigger,
            latest_tag: Some(v(0, 1, 0)),
            date: DATE,
            run_number: 42,
            signing: Signing::default(),
        }
    }

    fn dispatch(channel: &str, dry_run: bool, allow_unsigned: bool) -> Trigger {
        Trigger::Dispatch {
            channel: channel.to_owned(),
            dry_run,
            allow_unsigned,
        }
    }

    /// The semver precedence of two versions, as the updater compares them: numeric identifiers
    /// numerically, a pre-release below its release.
    fn semver_lt(a: &str, b: &str) -> bool {
        let split = |s: &str| {
            let (core, pre) = s.split_once('-').map_or((s, None), |(c, p)| (c, Some(p)));
            let core: Vec<u64> = core.split('.').map(|n| n.parse().unwrap()).collect();
            let pre: Option<Vec<String>> = pre.map(|p| p.split('.').map(str::to_owned).collect());
            (core, pre)
        };
        let ((ca, pa), (cb, pb)) = (split(a), split(b));
        if ca != cb {
            return ca < cb;
        }
        match (pa, pb) {
            (Some(_), None) => true,
            (None, _) => false,
            (Some(pa), Some(pb)) => {
                for (x, y) in pa.iter().zip(&pb) {
                    let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                        (Err(_), Err(_)) => x.cmp(y),
                    };
                    if order != std::cmp::Ordering::Equal {
                        return order.is_lt();
                    }
                }
                pa.len() < pb.len()
            }
        }
    }

    fn no_leading_zero(version: &str) -> bool {
        version
            .split(['.', '-'])
            .filter(|id| id.bytes().all(|b| b.is_ascii_digit()))
            .all(|id| id == "0" || !id.starts_with('0'))
    }

    #[test]
    fn a_tag_should_name_a_plain_version_within_the_msi_limits() {
        let table = [
            ("desktop-v0.1.0", Some(v(0, 1, 0))),
            ("desktop-v255.255.65535", Some(v(255, 255, 65_535))),
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
        assert_eq!(latest_tag(tags), Some(v(0, 10, 0)));
        assert_eq!(latest_tag(["junk"]), None);
    }

    #[test]
    fn a_nightly_should_be_the_next_patch_dated_and_numbered() {
        assert_eq!(
            nightly_version(Some(v(0, 1, 0)), DATE, 42).unwrap(),
            "0.1.1-nightly.20261028.42"
        );
        assert_eq!(nightly_version(None, DATE, 42).unwrap(), "0.1.0-nightly.20261028.42");
        assert!(nightly_version(Some(v(0, 1, 65_535)), DATE, 1).is_err());
        assert!(nightly_version(None, DATE, 0).is_err());
    }

    #[test]
    fn a_nightly_should_sort_after_the_last_release_and_before_the_next() {
        let after_tag = nightly_version(Some(v(0, 1, 0)), DATE, 42).unwrap();
        assert!(semver_lt("0.1.0", &after_tag));
        assert!(semver_lt(&after_tag, "0.1.1"));
        let before_tag = nightly_version(None, DATE, 42).unwrap();
        assert!(semver_lt("0.0.65535", &before_tag));
        assert!(semver_lt(&before_tag, "0.1.0"));
        // A later run, or a later day, sorts after.
        assert!(semver_lt(
            &after_tag,
            &nightly_version(Some(v(0, 1, 0)), DATE, 43).unwrap()
        ));
        assert!(semver_lt(
            &after_tag,
            &nightly_version(Some(v(0, 1, 0)), Date(20_261_029), 1).unwrap()
        ));
    }

    /// `^\d+\.\d+\.\d+-nightly\.[1-9]\d{7}\.[1-9]\d*$`, the todo's nightly pattern.
    fn is_nightly(version: &str) -> bool {
        let Some((core, pre)) = version.split_once("-nightly.") else {
            return false;
        };
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        let core_ok = core.split('.').count() == 3 && core.split('.').all(digits);
        match pre.split_once('.') {
            Some((date, run)) => {
                core_ok
                    && digits(date)
                    && date.len() == 8
                    && !date.starts_with('0')
                    && digits(run)
                    && !run.starts_with('0')
            }
            None => false,
        }
    }

    /// `^0\.0\.(10[1-9]|1[1-9]\d|[2-9]\d{2}|[1-9]\d{3,4})$`, the todo's staging pattern.
    fn is_staging(version: &str) -> bool {
        version
            .strip_prefix("0.0.")
            .and_then(number)
            .is_some_and(|patch| (101..=99_999).contains(&patch))
    }

    #[test]
    fn every_version_should_match_its_pattern_with_no_leading_zero() {
        for run in [1, 9, 10, 99, 100, 999, 65_435] {
            let n = nightly_version(Some(v(0, 1, 0)), DATE, run).unwrap();
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
    fn a_nightly_should_build_nsis_only_on_windows() {
        let plan = plan(&inputs(Trigger::Schedule)).unwrap();
        for leg in &plan.legs {
            let expected = if leg.os == "windows" { "nsis" } else { "" };
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
        let signed = Inputs {
            signing: Signing::parse("macos, windows").unwrap(),
            ..inputs(tag())
        };
        let plan = plan(&signed).unwrap();
        assert_eq!(
            (plan.version.as_str(), plan.environment),
            ("0.1.0", Some("desktop-release"))
        );
        assert_eq!(plan.destination, BETA);
        assert!(plan.legs.iter().all(|l| l.mode == Mode::Tag && l.bundles.is_empty()));
    }

    #[test]
    fn a_pre_release_tag_should_fail_the_plan() {
        let signed = Inputs {
            signing: Signing::parse("macos,windows").unwrap(),
            ..inputs(Trigger::Tag("desktop-v0.2.0-rc.1".to_owned()))
        };
        assert!(plan(&signed).is_err());
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
    fn unsigned_staging_should_need_the_explicit_input() {
        assert!(plan(&inputs(dispatch("staging", false, false))).is_err());
        let dry = plan(&inputs(dispatch("staging", true, false))).unwrap();
        assert!(dry.dry_run());
    }

    #[test]
    fn a_pull_request_should_dry_run_every_target_and_windows_as_a_tag() {
        let plan = plan(&inputs(Trigger::PullRequest)).unwrap();
        assert!(plan.dry_run());
        assert_eq!(plan.legs.len(), 7);
        let tag_legs: Vec<&Leg> = plan.legs.iter().filter(|l| l.mode == Mode::Tag).collect();
        assert_eq!(tag_legs.len(), 1);
        assert_eq!((tag_legs[0].platform, tag_legs[0].bundles), ("windows-x86_64", ""));
        assert!(Version::parse_plain(&tag_legs[0].version).is_ok());
    }

    #[test]
    fn signed_platforms_should_name_macos_or_windows() {
        assert_eq!(Signing::parse("").unwrap(), Signing::default());
        assert!(Signing::parse("linux").is_err());
    }
}
