//! The workflow lockfile's rules (todo X11, ADR 0019). `.github/workflows/actions.lock` locks every
//! action the workflows use, and the fast lane's `lockfile` job checks it on every pull request.
//! So `uses:` lines name release tags, never bare commit SHAs, and this repository's own actions
//! are written `$/`, which the lockfile follows, not `./`, which it doesn't.

use super::compile_cache::uses;
use super::{action_files, lines_of, needs, repository_file, workflow_files};
use crate::doctor::GH_ACTIONS_LOCK_VERSION;

const CI: &str = include_str!("../../../../.github/workflows/ci.yml");

/// Whether a `uses:` value pins a full commit SHA (`owner/repo@<40 hex digits>`).
fn pins_a_sha(value: &str) -> bool {
    value
        .rsplit_once('@')
        .is_some_and(|(_, at)| at.len() == 40 && at.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Every workflow and local action, by path, with its text.
fn workflows_and_actions() -> Vec<(String, String)> {
    workflow_files()
        .into_iter()
        .chain(action_files())
        .map(|path| {
            let text = repository_file(&path).unwrap_or_default();
            (path, text)
        })
        .collect()
}

/// Every `uses:` value in a workflow or local action that `offends`, as `path: value`.
fn uses_lines_where(offends: impl Fn(&str) -> bool) -> Vec<String> {
    workflows_and_actions()
        .iter()
        .flat_map(|(path, text)| {
            text.lines()
                .filter_map(uses)
                .filter(|v| offends(v))
                .map(move |v| format!("{path}: {v}"))
        })
        .collect()
}

#[test]
fn the_lockfile_job_should_verify_with_the_doctor_pin() {
    let job = lines_of(CI, "jobs", "lockfile").join("\n");
    assert!(
        job.contains("run: gh actions-lock --verify"),
        "the lockfile job must run --verify"
    );
    assert!(
        job.contains(&format!(
            "gh extension install github/gh-actions-lock --pin v{GH_ACTIONS_LOCK_VERSION}"
        )),
        "the lockfile job must install doctor.rs's GH_ACTIONS_LOCK_VERSION"
    );
    assert!(
        job.contains("GH_TOKEN: ${{ github.token }}"),
        "gh needs the job's token"
    );
}

/// It runs on every pull request, docs-only ones included: it has no `if:`, and the gate needs
/// it whatever the `changes` flags say.
#[test]
fn fast_gate_should_always_need_the_lockfile_job() {
    let job = lines_of(CI, "jobs", "lockfile");
    assert!(
        !job.iter().any(|l| l.trim_start().starts_with("if:")),
        "the lockfile job must not be skipped"
    );
    assert!(
        needs(CI, "fast-gate").contains(&"lockfile"),
        "fast-gate must need the lockfile job"
    );
    let gate = lines_of(CI, "jobs", "fast-gate").join("\n");
    assert!(
        gate.contains("lockfile) needed=true ;;"),
        "fast-gate must always need the lockfile job"
    );
}

#[test]
fn no_uses_line_should_pin_a_bare_sha() {
    assert_eq!(
        uses_lines_where(pins_a_sha),
        Vec::<String>::new(),
        "name the release tag; the lockfile holds the SHA"
    );
}

#[test]
fn local_actions_should_be_written_so_the_lockfile_follows_them() {
    assert_eq!(
        uses_lines_where(|v| v.starts_with("./.github/actions/")),
        Vec::<String>::new(),
        "write $/.github/actions/…, not ./"
    );
}

#[test]
fn pins_a_sha_should_tell_a_sha_from_a_tag() {
    assert_eq!(
        [
            pins_a_sha("actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"),
            pins_a_sha("actions/checkout@v7.0.1"),
            pins_a_sha("$/.github/actions/host-rust"),
        ],
        [true, false, false]
    );
}
