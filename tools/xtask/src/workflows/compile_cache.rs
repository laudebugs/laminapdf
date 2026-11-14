//! The compile cache's rules (docs/plans/ci-caching.md §4). `.github/actions/rust-compile-cache`
//! installs the sccache that `doctor.rs` pins. A job sets it up only after every toolchain step,
//! because the Rust cache hashes every `RUST*` variable into its key and a `RUSTC_WRAPPER` set
//! earlier would split the cache between jobs with and without the credentials. And nothing that
//! publishes, deploys or gates a release uses it, through any reusable workflow or local action
//! it calls, so a poisoned entry (wrong or planted) can't reach users or let a release through.
//!
//! The checks read `uses:` lines and mentions of sccache. They don't check the order
//! of steps inside a composite action, and they would miss sccache reached some other way, such as
//! a script that installs it.

use std::collections::VecDeque;

use super::{jobs, repository_file, workflow_files};

/// The action, as a step's `uses:` names it: `$/`, this repository's own action at the workflow's
/// commit, which the lockfile covers (todo X11).
const ACTION: &str = "$/.github/actions/rust-compile-cache";

/// The steps that install the toolchain and restore the Rust cache: a local action by its path,
/// a pinned one by the part before its `@`.
const TOOLCHAIN_STEPS: [&str; 2] = ["actions-rust-lang/setup-rust-toolchain", "$/.github/actions/host-rust"];

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

/// The value of a `uses:` line (a `- uses:` one too), without a trailing comment or quotes;
/// `None` for any other line, comments included.
pub(super) fn uses(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let line = line.strip_prefix("- ").unwrap_or(line).trim_start();
    let value = line.strip_prefix("uses:")?;
    Some(
        value
            .split(" #")
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(['"', '\'']),
    )
}

/// Whether a `uses:` value names `step`, at any pin.
fn names(value: &str, step: &str) -> bool {
    value
        .strip_prefix(step)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('@'))
}

/// The jobs of `workflow` that set the compile cache up before one of their toolchain steps, or
/// with none.
fn set_up_before_the_toolchain(workflow: &str) -> Vec<&str> {
    jobs(workflow)
        .into_iter()
        .filter(|(_, lines)| {
            let steps = || lines.iter().enumerate().filter_map(|(i, l)| uses(l).map(|v| (i, v)));
            let cache = steps().find(|(_, v)| names(v, ACTION)).map(|(i, _)| i);
            let toolchain = steps()
                .filter(|(_, v)| TOOLCHAIN_STEPS.iter().any(|t| names(v, t)))
                .map(|(i, _)| i)
                .next_back();
            cache.is_some_and(|cache| toolchain.is_none_or(|toolchain| cache < toolchain))
        })
        .map(|(id, _)| id)
        .collect()
}

/// The file a local `uses:` value loads, relative to the repository root: a reusable workflow
/// itself (`./`), or a local action's `action.yml` (`$/`, todo X11; `./` before it). `None` for an
/// action from another repository.
fn local_file(value: &str) -> Option<String> {
    let path = value.strip_prefix("$/").or_else(|| value.strip_prefix("./"))?;
    let workflow = std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e == "yml" || e == "yaml");
    Some(if workflow {
        path.to_owned()
    } else {
        format!("{path}/action.yml")
    })
}

/// Whether a line that isn't a whole-line comment names sccache: the action, `RUSTC_WRAPPER:
/// sccache`, an install of it. A trailing comment counts, which errs toward a failure.
fn mentions_sccache(line: &str) -> bool {
    !line.trim_start().starts_with('#') && (line.contains("sccache") || line.contains("rust-compile-cache"))
}

/// How `start` reaches sccache, through the reusable workflows and local actions it uses, as the
/// files on the way (`start` first, the one that mentions sccache last); `None` when it doesn't.
/// `read` gives a file's text by its path from the repository root.
fn path_to_sccache(start: &str, read: &dyn Fn(&str) -> Option<String>) -> Option<Vec<String>> {
    let mut queue = VecDeque::from([vec![start.to_owned()]]);
    let mut seen = vec![start.to_owned()];
    while let Some(route) = queue.pop_front() {
        let text = route.last().and_then(|file| read(file)).unwrap_or_default();
        if text.lines().any(mentions_sccache) {
            return Some(route);
        }
        for next in text.lines().filter_map(uses).filter_map(local_file) {
            if !seen.contains(&next) {
                seen.push(next.clone());
                let mut longer = route.clone();
                longer.push(next);
                queue.push_back(longer);
            }
        }
    }
    None
}

/// The sccache versions an action's text installs through `install-tools`.
fn installed_sccache(action: &str) -> Vec<&str> {
    action
        .lines()
        .filter_map(|l| l.trim().strip_prefix("tool: sccache@"))
        .map(|v| v.split(" #").next().unwrap_or_default().trim())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader over in-memory files, for fixtures.
    fn reader<'a>(files: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |path| {
            files
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, text)| (*text).to_owned())
        }
    }

    #[test]
    fn the_action_should_install_the_sccache_doctor_pins() {
        let action = repository_file(".github/actions/rust-compile-cache/action.yml").unwrap_or_default();
        assert_eq!(installed_sccache(&action), [crate::doctor::SCCACHE_VERSION]);
    }

    #[test]
    fn an_action_on_another_sccache_should_be_reported() {
        let action = "      with:\n        tool: sccache@0.18.0 # older\n";
        assert_ne!(installed_sccache(action), [crate::doctor::SCCACHE_VERSION]);
    }

    #[test]
    fn every_job_should_set_the_compile_cache_up_after_its_toolchain() {
        let early: Vec<String> = workflow_files()
            .iter()
            .flat_map(|path| {
                let text = repository_file(path).unwrap_or_default();
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

    /// The checks above look for ACTION by name: if no workflow names it (it was renamed, or its
    /// spelling changed, as `./` became `$/` in todo X11), they would pass by finding nothing.
    #[test]
    fn some_workflow_should_name_the_action_as_the_checks_spell_it() {
        let named = workflow_files().iter().any(|path| {
            repository_file(path).is_some_and(|text| text.lines().filter_map(uses).any(|v| names(v, ACTION)))
        });
        assert!(named, "no workflow names {ACTION}; update ACTION and TOOLCHAIN_STEPS");
    }

    #[test]
    fn nothing_published_deployed_or_released_should_use_sccache() {
        let all = workflow_files();
        let routes: Vec<Vec<String>> = NEVER_CACHED
            .iter()
            .map(|name| format!(".github/workflows/{name}"))
            .inspect(|path| assert!(all.contains(path), "{path} is gone; update NEVER_CACHED"))
            .filter_map(|path| path_to_sccache(&path, &repository_file))
            .collect();
        assert_eq!(routes, Vec::<Vec<String>>::new());
    }

    #[test]
    fn a_job_should_be_reported_when_it_sets_the_cache_up_before_a_toolchain() {
        let workflow = "jobs:
  rust:
    steps:
      - uses: actions/checkout@abc # v7
      - uses: \"$/.github/actions/rust-compile-cache\"
        with:
          bucket: b
      - uses: actions-rust-lang/setup-rust-toolchain@a188e68d # v2.0.1
  after:
    steps:
      - uses: $/.github/actions/host-rust
      - uses: $/.github/actions/rust-compile-cache
  second-toolchain:
    steps:
      - uses: $/.github/actions/host-rust
      - uses: $/.github/actions/rust-compile-cache
      - uses: $/.github/actions/host-rust
  without:
    steps:
      - uses: $/.github/actions/rust-compile-cache
  # - uses: $/.github/actions/rust-compile-cache
  none:
    steps:
      - run: cargo build
";
        assert_eq!(
            set_up_before_the_toolchain(workflow),
            ["rust", "second-toolchain", "without"]
        );
    }

    #[test]
    fn a_step_should_be_named_at_any_pin_but_not_by_a_longer_name() {
        assert_eq!(
            [
                names("actions-rust-lang/setup-rust-toolchain@a188e68d", TOOLCHAIN_STEPS[0]),
                names("$/.github/actions/host-rust", TOOLCHAIN_STEPS[1]),
                names("$/.github/actions/host-rust-x", TOOLCHAIN_STEPS[1]),
            ],
            [true, true, false]
        );
    }

    #[test]
    fn a_comment_should_not_count_as_a_step() {
        assert_eq!(uses("      # - uses: $/.github/actions/rust-compile-cache"), None);
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
                "jobs:\n  fuzz:\n    steps:\n      - uses: $/.github/actions/host-rust\n      - uses: $/.github/actions/rust-compile-cache\n",
            ),
        ];
        assert_eq!(
            path_to_sccache(".github/workflows/release.yml", &reader(&files)),
            Some(vec![
                ".github/workflows/release.yml".to_owned(),
                ".github/workflows/nightly.yml".to_owned(),
            ])
        );
    }

    #[test]
    fn a_local_action_that_sets_the_wrapper_itself_should_be_reported() {
        let files = [
            (
                ".github/workflows/docs.yml",
                "jobs:\n  build:\n    steps:\n      - uses: $/.github/actions/build-rust\n",
            ),
            (
                ".github/actions/build-rust/action.yml",
                "runs:\n  steps:\n    - run: echo \"RUSTC_WRAPPER=sccache\" >> \"$GITHUB_ENV\"\n",
            ),
        ];
        assert!(path_to_sccache(".github/workflows/docs.yml", &reader(&files)).is_some());
    }

    #[test]
    fn a_workflow_that_never_reaches_sccache_should_pass_even_with_a_cycle_and_a_comment() {
        let files = [
            (".github/workflows/a.yml", "    uses: ./.github/workflows/b.yml\n"),
            (
                ".github/workflows/b.yml",
                "    # never sccache here (ADR 0018)\n    uses: ./.github/workflows/a.yml\n",
            ),
        ];
        assert_eq!(path_to_sccache(".github/workflows/a.yml", &reader(&files)), None);
    }
}
