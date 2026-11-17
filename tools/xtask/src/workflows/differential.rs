//! The live differentials can't go stale on pull requests. Every mode of
//! [`crate::live::Mode::ALL`] runs in the nightly's `differential` job; the full lane's
//! `live-differentials` job runs it with `--sample` on its own `differential-<mode>` flag, which the
//! `changes` job exports and pushes to main set; the job passes that flag to the shared setup; and
//! the mode's exceptions files exist. A mode added to the xtask is covered at once, and one the
//! nightly runs with no PR-lane step fails here.

use super::{lines_of, repository_file};
use crate::live::Mode;

const NIGHTLY: &str = ".github/workflows/nightly.yml";
const FULL: &str = ".github/workflows/full.yml";

/// The `differential` arguments of each `cargo xtask differential` run in `lines`, up to the first
/// that is neither a flag nor `--sample` (`--live --text`), with whether the run passes
/// `--sample`.
fn differential_runs(lines: &[&str]) -> Vec<(String, bool)> {
    lines
        .iter()
        .filter_map(|line| line.split_once("cargo xtask differential "))
        .map(|(_, rest)| {
            let args: Vec<&str> = rest
                .split_whitespace()
                .take_while(|arg| arg.starts_with("--") && *arg != "--sample")
                .collect();
            (args.join(" "), rest.contains("--sample"))
        })
        .collect()
}

/// The `if:` line of the step that holds line `run` of a job's `lines`.
fn condition_of<'a>(lines: &[&'a str], run: usize) -> Option<&'a str> {
    let step = lines.get(..=run)?;
    let start = step.iter().rposition(|l| l.trim_start().starts_with("- "))?;
    step.get(start..)?
        .iter()
        .map(|l| l.trim_start().trim_start_matches("- "))
        .find(|l| l.starts_with("if:"))
}

/// What is missing for `mode` in the two lanes, as messages.
fn gaps(nightly: &str, full: &str, mode: Mode) -> Vec<String> {
    let args = mode.args();
    let flag = format!("differential-{}", mode.name());
    let mut found = Vec::new();
    let night = lines_of(nightly, "jobs", "differential");
    if !differential_runs(&night).iter().any(|(a, _)| *a == args) {
        found.push(format!(
            "nightly.yml's differential job doesn't run `differential {args}`"
        ));
    }
    let job = lines_of(full, "jobs", "live-differentials");
    let sampled = job.iter().enumerate().find(|(_, line)| {
        differential_runs(&[line])
            .first()
            .is_some_and(|(a, sample)| *a == args && *sample)
    });
    match sampled {
        None => found.push(format!(
            "full.yml's live-differentials job has no `differential {args} --sample` step"
        )),
        Some((at, _)) => {
            let guard = format!("needs.changes.outputs.{flag} == 'true'");
            if !condition_of(&job, at).is_some_and(|c| c.contains(&guard)) {
                found.push(format!(
                    "the `differential {args}` step in full.yml should run on `{guard}`"
                ));
            }
        }
    }
    let setup = format!("{}: ${{{{ needs.changes.outputs.{flag} }}}}", mode.name());
    if !job.iter().any(|l| l.trim() == setup) {
        found.push(format!(
            "live-differentials should pass `{setup}` to differential-tools"
        ));
    }
    let changes = lines_of(full, "jobs", "changes");
    let output = format!("{flag}: ${{{{ steps.pr.outputs.{flag} || steps.push.outputs.{flag} }}}}");
    if !changes.iter().any(|l| l.trim() == output) {
        found.push(format!("the changes job should export `{output}`"));
    }
    if !changes.iter().any(|l| l.trim() == format!("echo \"{flag}=true\"")) {
        found.push(format!("pushes to main should set {flag}=true"));
    }
    for file in mode.exceptions() {
        if repository_file(file).is_none() {
            found.push(format!(
                "{file}, the exceptions of `differential {args}`, doesn't exist"
            ));
        }
    }
    found
}

/// Every mode the xtask has runs in the nightly, has a sampled PR-lane step on its own flag, and
/// has its exceptions files.
#[test]
fn every_live_differential_should_run_sampled_on_pull_requests() {
    let nightly = repository_file(NIGHTLY).expect("nightly.yml");
    let full = repository_file(FULL).expect("full.yml");
    let missing: Vec<String> = Mode::ALL
        .into_iter()
        .flat_map(|mode| gaps(&nightly, &full, mode))
        .collect();
    assert_eq!(missing, Vec::<String>::new());
}

/// No nightly run names a mode the xtask doesn't list, so the list above is the whole set.
#[test]
fn every_nightly_live_differential_should_be_a_mode() {
    let nightly = repository_file(NIGHTLY).expect("nightly.yml");
    let known: Vec<String> = Mode::ALL.map(Mode::args).to_vec();
    let unknown: Vec<String> = differential_runs(&lines_of(&nightly, "jobs", "differential"))
        .into_iter()
        .map(|(args, _)| args)
        .filter(|args| args.starts_with("--live") && !known.contains(args))
        .collect();
    assert_eq!(unknown, Vec::<String>::new());
}

#[test]
fn gaps_should_report_a_nightly_mode_without_a_sampled_step() {
    let nightly = "jobs:\n  differential:\n    steps:\n      - run: cargo xtask differential --live --xmp\n";
    let full = "jobs:\n  live-differentials:\n    steps:\n      - run: cargo xtask differential --live --xmp\n";
    let found = gaps(nightly, full, Mode::Xmp);
    assert!(
        found
            .iter()
            .any(|g| g.contains("no `differential --live --xmp --sample` step")),
        "{found:?}"
    );
}

#[test]
fn gaps_should_report_a_step_on_another_modes_flag() {
    let nightly = "jobs:\n  differential:\n    steps:\n      - run: cargo xtask differential --live --text\n";
    let full = "jobs:\n  live-differentials:\n    steps:\n      - name: Text\n        if: ${{ needs.changes.outputs.differential-xmp == 'true' }}\n        run: cargo xtask differential --live --text --sample \"$SAMPLE\"\n";
    let found = gaps(nightly, full, Mode::Text);
    assert!(found.iter().any(|g| g.contains("should run on")), "{found:?}");
}

#[test]
fn differential_runs_should_stop_at_the_sample() {
    assert_eq!(
        differential_runs(&[
            "        run: cargo xtask differential --live --text ${DIFFERENTIAL_SAMPLE:+--sample \"$X\"}",
            "        run: cargo xtask differential --live --sample \"$SAMPLE\"",
        ]),
        [("--live --text".to_owned(), true), ("--live".to_owned(), true)]
    );
}
