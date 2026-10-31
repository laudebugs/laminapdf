//! `cargo xtask release-guard`: the release lane's first job refuses to release while the
//! nightly lane is red (ADR 0018 §2: "a nightly failure … blocks the next release"; ADR 0017).
//!
//! It asks GitHub (`gh`, authenticated by `GH_TOKEN` in Actions) for the latest completed run of
//! `nightly.yml` on `main`, and passes only when that run succeeded. `--run <id>` checks one given
//! run instead, which is how the guard is tested against a known failed run.

use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::program_path;

/// The fields read from `gh run list` / `gh run view`.
const FIELDS: &str = "databaseId,conclusion,status,headSha,headBranch,workflowName,url,createdAt";
const USAGE: &str = "usage: cargo xtask release-guard [--run <run-id>]";

pub(crate) fn parse_args(args: &[String]) -> Result<Option<String>> {
    match args {
        [] => Ok(None),
        [flag, id] if flag == "--run" && !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) => {
            Ok(Some(id.clone()))
        }
        _ => bail!(USAGE),
    }
}

/// Checks the latest completed nightly run on `main` (or run `run`) and fails unless it succeeded.
pub(crate) fn guard(run: Option<&str>) -> Result<()> {
    crate::step("release-guard: the latest nightly run on main must have succeeded");
    let json = match run {
        Some(id) => gh(&["run", "view", id, "--json", FIELDS])?,
        None => gh(&[
            "run",
            "list",
            "--workflow",
            "nightly.yml",
            "--branch",
            "main",
            "--status",
            "completed",
            "--limit",
            "1",
            "--json",
            FIELDS,
        ])?,
    };
    let verdict = verdict(&json)?;
    println!("{verdict}");
    Ok(())
}

fn gh(args: &[&str]) -> Result<String> {
    let output = Command::new(program_path("gh"))
        .args(args)
        .output()
        .context("failed to run gh; is the GitHub CLI installed (and GH_TOKEN set in Actions)?")?;
    if !output.status.success() {
        bail!(
            "`gh {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("gh printed non-UTF-8 output")
}

/// The decision for `gh`'s JSON: one run object (`run view`) or a list of at most one (`run
/// list`). `Ok` with a line to print when the run is a successful nightly run; otherwise the
/// reason the release is refused.
fn verdict(json: &str) -> Result<String> {
    let value: Value = serde_json::from_str(json).context("gh printed JSON this guard can't read")?;
    let run = match &value {
        Value::Array(runs) => runs
            .first()
            .context("refusing to release: there is no completed nightly run on main (run nightly.yml first)")?,
        Value::Object(_) => &value,
        _ => bail!("gh printed neither a run nor a list of runs"),
    };
    let field = |name: &str| run.get(name).and_then(Value::as_str).unwrap_or("?");
    let id = run.get("databaseId").and_then(Value::as_u64).unwrap_or_default();
    let what = format!(
        "nightly run {id} on {} ({}, {}): {}",
        field("headBranch"),
        field("headSha"),
        field("createdAt"),
        field("url")
    );
    if field("workflowName") != "Nightly" {
        bail!(
            "refusing to release: run {id} is a run of {:?}, not the nightly lane",
            field("workflowName")
        );
    }
    if field("status") != "completed" {
        bail!("refusing to release: {what} has not finished ({})", field("status"));
    }
    match field("conclusion") {
        "success" => Ok(format!("ok: {what} succeeded")),
        conclusion => bail!(
            "refusing to release: {what} ended with {conclusion:?}; fix the nightly lane (its issue is \
             labelled `nightly`) and run it again (ADR 0018 §2)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, verdict};

    fn run(conclusion: &str, status: &str, workflow: &str) -> String {
        format!(
            r#"{{"databaseId":42,"conclusion":"{conclusion}","status":"{status}","headSha":"abc","headBranch":"main",
               "workflowName":"{workflow}","url":"https://example.invalid/42","createdAt":"2026-10-30T03:17:00Z"}}"#
        )
    }

    #[test]
    fn verdict_should_pass_a_successful_nightly_run() {
        assert!(verdict(&format!("[{}]", run("success", "completed", "Nightly"))).is_ok());
    }

    #[test]
    fn verdict_should_refuse_a_failed_nightly_run() {
        let error = verdict(&format!("[{}]", run("failure", "completed", "Nightly"))).unwrap_err();
        assert!(error.to_string().contains("ended with \"failure\""), "{error}");
    }

    #[test]
    fn verdict_should_refuse_cancelled_and_unfinished_runs() {
        let refused = [
            run("cancelled", "completed", "Nightly"),
            run("", "in_progress", "Nightly"),
        ]
        .map(|r| verdict(&r).is_err());
        assert_eq!(refused, [true, true]);
    }

    #[test]
    fn verdict_should_refuse_when_there_is_no_run() {
        assert!(verdict("[]").is_err());
    }

    #[test]
    fn verdict_should_refuse_a_run_of_another_workflow() {
        assert!(verdict(&run("success", "completed", "Full lane")).is_err());
    }

    /// The release lane publishes nothing (ADR 0017 §6; publishing is todo P1): no step of the
    /// workflow may run a publish command or a release action. Comments may name them.
    #[test]
    fn release_workflow_should_have_no_publish_step() {
        const WORKFLOW: &str = include_str!("../../../.github/workflows/release.yml");
        let forbidden = [
            "npm publish",
            "pnpm publish",
            "cargo publish",
            "changeset publish",
            "changeset tag",
            "gh release",
            "action-gh-release",
            "git tag",
            "git push",
            "id-token",
        ];
        let found: Vec<&str> = WORKFLOW
            .lines()
            .map(|line| line.split(" #").next().unwrap_or_default())
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter(|line| forbidden.iter().any(|f| line.contains(f)))
            .collect();
        assert_eq!(found, Vec::<&str>::new());
    }

    #[test]
    fn parse_args_should_take_an_optional_numeric_run_id() {
        let parse = |a: &[&str]| parse_args(&a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()).ok();
        assert_eq!(
            [parse(&[]), parse(&["--run", "123"]), parse(&["--run", "x"])],
            [Some(None), Some(Some("123".to_owned())), None]
        );
    }
}
