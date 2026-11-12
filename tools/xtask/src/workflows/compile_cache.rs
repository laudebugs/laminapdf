//! The compile cache's rules (docs/plans/ci-caching.md §4). `.github/actions/rust-compile-cache`
//! installs the sccache that `doctor.rs` pins. A job sets it up only after its toolchain step,
//! because the Rust cache hashes every `RUST*` variable into its key and a `RUSTC_WRAPPER` set
//! earlier would split the cache between jobs with and without the credentials. And nothing that
//! is published, deployed or gates a release uses it, through any reusable workflow or local
//! action it calls, so a poisoned entry can't reach users or wave a release through.

use std::collections::VecDeque;
use std::path::Path;

use super::jobs;

/// The action, as a step's `uses:` names it.
const ACTION: &str = "./.github/actions/rust-compile-cache";

/// The steps that install the toolchain and restore the Rust cache.
const TOOLCHAIN_STEPS: [&str; 2] = ["actions-rust-lang/setup-rust-toolchain@", "./.github/actions/host-rust"];

/// The workflows that publish, deploy or gate a release (plan §4.3).
const NEVER_CACHED: [&str; 8] = [
    "release.yml",
    "cli-release.yml",
    "release-train.yml",
    "web-deploy.yml",
    "web-release-train.yml",
    "docs.yml",
    "site.yml",
    "nightly.yml",
];

/// The value of a `uses:` line (a `- uses:` one too), without a trailing comment; `None` for any
/// other line, comments included.
fn uses(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let line = line.strip_prefix("- ").unwrap_or(line).trim_start();
    let value = line.strip_prefix("uses:")?;
    Some(value.split(" #").next().unwrap_or_default().trim())
}

/// Whether a step's `uses:` value names `target`: the action's path exactly, or a pinned action
/// by its `owner/repo@` prefix.
fn names(value: &str, target: &str) -> bool {
    if target.ends_with('@') {
        value.starts_with(target)
    } else {
        value == target
    }
}

/// The jobs of `workflow` that set the compile cache up with no toolchain step before it.
fn set_up_before_the_toolchain(workflow: &str) -> Vec<&str> {
    jobs(workflow)
        .into_iter()
        .filter(|(_, lines)| {
            let at = |targets: &[&str]| {
                lines
                    .iter()
                    .position(|l| uses(l).is_some_and(|v| targets.iter().any(|t| names(v, t))))
            };
            match (at(&[ACTION]), at(&TOOLCHAIN_STEPS)) {
                (Some(cache), Some(toolchain)) => cache < toolchain,
                (Some(_), None) => true,
                (None, _) => false,
            }
        })
        .map(|(id, _)| id)
        .collect()
}

fn is_yaml(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e == "yml" || e == "yaml")
}

/// The file a local `uses:` value loads, relative to the repository root: a reusable workflow
/// itself, or a local action's `action.yml`. `None` for an action from another repository.
fn local_file(value: &str) -> Option<String> {
    let path = value.strip_prefix("./")?;
    if is_yaml(path) {
        Some(path.to_owned())
    } else {
        Some(format!("{path}/action.yml"))
    }
}

/// How `start` reaches the compile cache's action, through the reusable workflows and local
/// actions it uses, as the files on the way (`start` first, the action's last); `None` when it
/// doesn't. `read` gives a file's text by its path from the repository root.
fn path_to_the_cache(start: &str, read: &dyn Fn(&str) -> Option<String>) -> Option<Vec<String>> {
    let target = local_file(ACTION)?;
    let mut queue = VecDeque::from([vec![start.to_owned()]]);
    let mut seen = vec![start.to_owned()];
    while let Some(route) = queue.pop_front() {
        let text = route.last().and_then(|file| read(file)).unwrap_or_default();
        for next in text.lines().filter_map(uses).filter_map(local_file) {
            let mut longer = route.clone();
            longer.push(next.clone());
            if next == target {
                return Some(longer);
            }
            if !seen.contains(&next) {
                seen.push(next);
                queue.push_back(longer);
            }
        }
    }
    None
}

fn repository() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read_from_repository(path: &str) -> Option<String> {
    std::fs::read_to_string(repository().join(path)).ok()
}

/// Every workflow file's path from the repository root.
fn workflows() -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(repository().join(".github/workflows"))
        .map(|dir| dir.filter_map(Result::ok).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_yaml(name))
        .map(|name| format!(".github/workflows/{name}"))
        .collect();
    found.sort();
    found
}

#[test]
fn the_action_should_install_the_sccache_doctor_pins() {
    let action = read_from_repository(".github/actions/rust-compile-cache/action.yml").unwrap_or_default();
    let installed: Vec<&str> = action
        .lines()
        .filter_map(|l| l.trim().strip_prefix("tool: sccache@"))
        .map(|v| v.split(" #").next().unwrap_or_default().trim())
        .collect();
    assert_eq!(installed, [crate::doctor::SCCACHE_VERSION]);
}

#[test]
fn every_job_should_set_the_compile_cache_up_after_its_toolchain() {
    let early: Vec<String> = workflows()
        .iter()
        .flat_map(|path| {
            let text = read_from_repository(path).unwrap_or_default();
            set_up_before_the_toolchain(&text)
                .into_iter()
                .map(|job| format!("{path}: {job}"))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        early,
        Vec::<String>::new(),
        "move the compile cache after the toolchain step"
    );
}

#[test]
fn nothing_published_deployed_or_released_should_use_the_compile_cache() {
    let all = workflows();
    let routes: Vec<Vec<String>> = NEVER_CACHED
        .iter()
        .map(|name| format!(".github/workflows/{name}"))
        .inspect(|path| assert!(all.contains(path), "{path} is gone; update NEVER_CACHED"))
        .filter_map(|path| path_to_the_cache(&path, &read_from_repository))
        .collect();
    assert_eq!(routes, Vec::<Vec<String>>::new());
}

#[test]
fn a_job_should_be_reported_when_it_sets_the_cache_up_before_the_toolchain() {
    let workflow = "jobs:
  rust:
    steps:
      - uses: actions/checkout@abc # v7
      - uses: ./.github/actions/rust-compile-cache
        with:
          bucket: b
      - uses: actions-rust-lang/setup-rust-toolchain@a188e68d # v2.0.1
  after:
    steps:
      - uses: ./.github/actions/host-rust
      - uses: ./.github/actions/rust-compile-cache
  without:
    steps:
      - uses: ./.github/actions/rust-compile-cache
  # - uses: ./.github/actions/rust-compile-cache
  none:
    steps:
      - run: cargo build
";
    assert_eq!(set_up_before_the_toolchain(workflow), ["rust", "without"]);
}

#[test]
fn a_comment_should_not_count_as_a_step() {
    assert_eq!(uses("      # - uses: ./.github/actions/rust-compile-cache"), None);
}

#[test]
fn a_nightly_that_uses_the_cache_should_be_reported_through_the_release_lane() {
    let files = [
        (
            ".github/workflows/release.yml",
            "jobs:\n  nightly:\n    uses: ./.github/workflows/nightly.yml # the nightly suites\n",
        ),
        (
            ".github/workflows/nightly.yml",
            "jobs:\n  fuzz:\n    steps:\n      - uses: ./.github/actions/host-rust\n      - uses: ./.github/actions/rust-compile-cache\n",
        ),
    ];
    let read = |path: &str| {
        files
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, text)| (*text).to_owned())
    };
    assert_eq!(
        path_to_the_cache(".github/workflows/release.yml", &read),
        Some(vec![
            ".github/workflows/release.yml".to_owned(),
            ".github/workflows/nightly.yml".to_owned(),
            ".github/actions/rust-compile-cache/action.yml".to_owned(),
        ])
    );
}

#[test]
fn a_local_action_that_uses_the_cache_should_be_reported() {
    let files = [
        (
            ".github/workflows/docs.yml",
            "jobs:\n  build:\n    steps:\n      - uses: ./.github/actions/build-rust\n",
        ),
        (
            ".github/actions/build-rust/action.yml",
            "runs:\n  steps:\n    - uses: ./.github/actions/rust-compile-cache\n",
        ),
    ];
    let read = |path: &str| {
        files
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, text)| (*text).to_owned())
    };
    assert!(path_to_the_cache(".github/workflows/docs.yml", &read).is_some());
}

#[test]
fn a_workflow_that_never_reaches_the_cache_should_pass_even_with_a_cycle() {
    let files = [
        (".github/workflows/a.yml", "    uses: ./.github/workflows/b.yml\n"),
        (".github/workflows/b.yml", "    uses: ./.github/workflows/a.yml\n"),
    ];
    let read = |path: &str| {
        files
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, text)| (*text).to_owned())
    };
    assert_eq!(path_to_the_cache(".github/workflows/a.yml", &read), None);
}
