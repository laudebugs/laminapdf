//! `cargo xtask release-guard`: the release lane's first job refuses to release while the
//! nightly lane is red (ADR 0018 §2: "a nightly failure … blocks the next release"; ADR 0017).
//!
//! It asks GitHub (`gh`, authenticated by `GH_TOKEN` in Actions) for the latest completed
//! **scheduled** run of `nightly.yml` on `main` (a dispatched run may have a short fuzz budget or a
//! sampled differential, so it doesn't count), and passes only when that run succeeded on the
//! release commit or one of its ancestors (`git merge-base --is-ancestor`; the release lane checks
//! out full history). `--run <id>` checks one given run instead, under the same rules, which is
//! how the guard is tested against a known failed run; `--commit <rev>` names the release commit
//! (default `HEAD`).

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::program_path;
use crate::util::git_stdout;

/// The fields read from `gh run list` / `gh run view`.
const FIELDS: &str = "databaseId,conclusion,status,event,headSha,headBranch,workflowName,url,createdAt";
const USAGE: &str = "usage: cargo xtask release-guard [--run <run-id>] [--commit <rev>]";

/// What `cargo xtask release-guard` was asked to check.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Args {
    run: Option<String>,
    commit: String,
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        run: None,
        commit: "HEAD".to_owned(),
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match (arg.as_str(), rest.next()) {
            ("--run", Some(id)) if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) => {
                parsed.run = Some(id.clone());
            }
            ("--commit", Some(rev)) if !rev.is_empty() => parsed.commit.clone_from(rev),
            _ => bail!(USAGE),
        }
    }
    Ok(parsed)
}

/// Checks the latest completed scheduled nightly run on `main` (or run `run`) and fails unless
/// it succeeded on the release commit or an ancestor of it.
pub(crate) fn guard(root: &Path, args: &Args) -> Result<()> {
    crate::step("release-guard: the latest scheduled nightly run on main must have succeeded");
    let release = git_stdout(root, &["rev-parse", "--verify", &format!("{}^{{commit}}", args.commit)])
        .with_context(|| format!("{:?} is not a commit in this clone", args.commit))?;
    let json = match &args.run {
        Some(id) => gh(&["run", "view", id, "--json", FIELDS])?,
        None => gh(&[
            "run",
            "list",
            "--workflow",
            "nightly.yml",
            "--branch",
            "main",
            "--event",
            "schedule",
            "--status",
            "completed",
            "--limit",
            "1",
            "--json",
            FIELDS,
        ])?,
    };
    let verdict = verdict(&json, &release, &|nightly: &str| is_ancestor(root, nightly, &release))?;
    println!("{verdict}");
    Ok(())
}

/// Whether `ancestor` is `commit` or one of its ancestors; an error when git can't tell (a commit
/// missing from a shallow clone).
fn is_ancestor(root: &Path, ancestor: &str, commit: &str) -> Result<bool> {
    let status = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, commit])
        .current_dir(root)
        .status()
        .context("failed to run git")?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => bail!("git merge-base --is-ancestor {ancestor} {commit} failed: is the clone shallow? (fetch-depth: 0)"),
    }
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
/// list`). `Ok` with a line to print when the run is a successful scheduled nightly run on `main`
/// whose commit `contains` says is in the release commit's history; otherwise the reason the
/// release is refused.
fn verdict(json: &str, release: &str, contains: &dyn Fn(&str) -> Result<bool>) -> Result<String> {
    let value: Value = serde_json::from_str(json).context("gh printed JSON this guard can't read")?;
    let run = match &value {
        Value::Array(runs) => runs.first().context(
            "refusing to release: there is no completed scheduled nightly run on main (wait for the next night)",
        )?,
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
    if field("headBranch") != "main" {
        bail!("refusing to release: {what} is not a run on main");
    }
    if field("event") != "schedule" {
        bail!(
            "refusing to release: {what} was triggered by {:?}; only scheduled nights count (a dispatched \
             run may use short budgets)",
            field("event")
        );
    }
    if field("status") != "completed" {
        bail!("refusing to release: {what} has not finished ({})", field("status"));
    }
    if field("conclusion") != "success" {
        bail!(
            "refusing to release: {what} ended with {:?}; fix the nightly lane (its issue is labelled \
             `nightly`) and wait for a green night (ADR 0018 §2)",
            field("conclusion")
        );
    }
    if !contains(field("headSha"))? {
        bail!("refusing to release: {what} tested a commit that is not in the history of the release commit {release}");
    }
    Ok(format!("ok: {what} succeeded on an ancestor of {release}"))
}

/// `cargo xtask release-notes <tag> <out>`: the root `CHANGELOG.md`'s section for the tag's
/// version (`v0.2.0-alpha.1` → `## 0.2.0-alpha.1`), without its heading, into `out`: the body of
/// the GitHub release the release lane creates (ADR 0017 §7).
pub(crate) fn release_notes(root: &Path, tag: &str, out: &Path) -> Result<()> {
    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md")).context("reading CHANGELOG.md")?;
    let notes = notes_for(&changelog, tag)?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(out, notes).with_context(|| format!("writing {}", out.display()))
}

/// The section of `changelog` under `## <version of tag>`, up to the next `## ` heading.
fn notes_for(changelog: &str, tag: &str) -> Result<String> {
    let version = tag
        .strip_prefix('v')
        .with_context(|| format!("{tag} is not a v<version> tag"))?;
    let heading = format!("## {version}");
    let mut lines = changelog.lines().skip_while(|line| *line != heading);
    if lines.next().is_none() {
        bail!("CHANGELOG.md has no `{heading}` section (run `pnpm changeset version` and `cargo xtask version sync`)");
    }
    let body: Vec<&str> = lines.take_while(|line| !line.starts_with("## ")).collect();
    Ok(format!("{}\n", body.join("\n").trim()))
}

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use super::{Args, notes_for, parse_args, verdict};

    fn run_with(conclusion: &str, status: &str, workflow: &str, branch: &str, event: &str) -> String {
        format!(
            r#"{{"databaseId":42,"conclusion":"{conclusion}","status":"{status}","event":"{event}","headSha":"abc",
               "headBranch":"{branch}","workflowName":"{workflow}","url":"https://example.invalid/42",
               "createdAt":"2026-10-30T03:17:00Z"}}"#
        )
    }

    fn run(conclusion: &str, status: &str, workflow: &str) -> String {
        run_with(conclusion, status, workflow, "main", "schedule")
    }

    #[expect(clippy::unnecessary_wraps, reason = "the signature `verdict` takes")]
    fn in_history(_: &str) -> Result<bool> {
        Ok(true)
    }

    #[expect(clippy::unnecessary_wraps, reason = "the signature `verdict` takes")]
    fn not_in_history(_: &str) -> Result<bool> {
        Ok(false)
    }

    #[test]
    fn verdict_should_pass_a_successful_scheduled_nightly_run_on_an_ancestor() {
        assert!(
            verdict(
                &format!("[{}]", run("success", "completed", "Nightly")),
                "def",
                &in_history
            )
            .is_ok()
        );
    }

    #[test]
    fn verdict_should_refuse_a_failed_nightly_run() {
        let error = verdict(
            &format!("[{}]", run("failure", "completed", "Nightly")),
            "def",
            &in_history,
        )
        .unwrap_err();
        assert!(error.to_string().contains("ended with \"failure\""), "{error}");
    }

    #[test]
    fn verdict_should_refuse_cancelled_and_unfinished_runs() {
        let refused = [
            run("cancelled", "completed", "Nightly"),
            run("", "in_progress", "Nightly"),
        ]
        .map(|r| verdict(&r, "def", &in_history).is_err());
        assert_eq!(refused, [true, true]);
    }

    #[test]
    fn verdict_should_refuse_when_there_is_no_run() {
        assert!(verdict("[]", "def", &in_history).is_err());
    }

    #[test]
    fn verdict_should_refuse_a_run_of_another_workflow() {
        assert!(verdict(&run("success", "completed", "Full lane"), "def", &in_history).is_err());
    }

    #[test]
    fn verdict_should_refuse_a_run_on_another_branch() {
        let error = verdict(
            &run_with("success", "completed", "Nightly", "ci/x3c-link-check", "push"),
            "def",
            &in_history,
        )
        .unwrap_err();
        assert!(error.to_string().contains("not a run on main"), "{error}");
    }

    #[test]
    fn verdict_should_refuse_a_dispatched_run() {
        let error = verdict(
            &run_with("success", "completed", "Nightly", "main", "workflow_dispatch"),
            "def",
            &in_history,
        )
        .unwrap_err();
        assert!(error.to_string().contains("only scheduled nights count"), "{error}");
    }

    #[test]
    fn verdict_should_refuse_a_run_on_a_commit_outside_the_release_history() {
        let error = verdict(&run("success", "completed", "Nightly"), "def", &not_in_history).unwrap_err();
        assert!(error.to_string().contains("not in the history"), "{error}");
    }

    #[test]
    fn is_ancestor_should_tell_ancestors_from_later_commits() {
        // A scratch repository: CI's checkout is shallow, so the real one may have no HEAD~1.
        let tree = crate::tracked::TempTree::new("release-ancestor", &[("a.txt", "a")]);
        let root = tree.0.as_path();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(root)
                .status()
                .is_ok_and(|s| s.success());
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "a.txt"]);
        git(&["commit", "-q", "-m", "first"]);
        git(&["commit", "-q", "--allow-empty", "-m", "second"]);
        let head = crate::util::git_stdout(root, &["rev-parse", "HEAD"]).unwrap();
        let parent = crate::util::git_stdout(root, &["rev-parse", "HEAD~1"]).unwrap();
        assert_eq!(
            [
                super::is_ancestor(root, &parent, &head).unwrap(),
                super::is_ancestor(root, &head, &parent).unwrap()
            ],
            [true, false]
        );
    }

    /// What publishes for real only on a tag push: `on_tag` in a step's `if:`, the token or
    /// `DRY_RUN` behind it.
    const ON_TAG: &str = "github.event_name == 'push' && github.ref_type == 'tag'";

    /// The steps of a job (each `      - …` item with its lines), comment lines left out.
    fn steps<'a>(lines: &[&'a str]) -> Vec<Vec<&'a str>> {
        let mut steps: Vec<Vec<&str>> = Vec::new();
        for line in lines.iter().copied().filter(|l| !l.trim_start().starts_with('#')) {
            if line.starts_with("      - ") {
                steps.push(vec![line]);
            } else if let Some(step) = steps.last_mut() {
                step.push(line);
            }
        }
        steps
    }

    /// The release lane publishes only from its publish jobs (ADR 0017 §7, todo P1): no other job
    /// runs a publish command, asks for an OIDC token or creates a release; each publish job waits
    /// for every check; and only a tag push publishes, stages or releases for real, a dispatch
    /// (even on a tag) being a dry run. Comments may name anything.
    #[test]
    fn release_workflow_should_publish_only_from_its_publish_jobs_and_only_on_a_tag_push() {
        const WORKFLOW: &str = include_str!("../../../.github/workflows/release.yml");
        const PUBLISH_JOBS: [&str; 3] = ["publish-crates", "publish-npm", "github-release"];
        const PUBLISHING: [&str; 13] = [
            "npm publish",
            "pnpm publish",
            "cargo publish",
            "changeset publish",
            "changeset tag",
            "xtask publish",
            "gh release",
            "action-gh-release",
            "crates-io-auth-action",
            "NPM_TOKEN",
            "git tag",
            "git push",
            "id-token",
        ];
        let jobs = crate::workflows::jobs(WORKFLOW);
        assert!(PUBLISH_JOBS.iter().all(|job| jobs.iter().any(|(id, _)| id == job)));
        for (job, lines) in &jobs {
            let code: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|l| !l.trim_start().starts_with('#'))
                .collect();
            let publishing: Vec<&str> = code
                .iter()
                .copied()
                .filter(|l| PUBLISHING.iter().any(|p| l.contains(p)))
                .collect();
            if !PUBLISH_JOBS.contains(job) {
                assert!(publishing.is_empty(), "{job} publishes: {publishing:?}");
                continue;
            }
            let text = code.join("\n");
            assert!(
                text.contains("needs.release.result == 'success'")
                    || text.contains("needs.publish-crates.result == 'success'"),
                "{job} doesn't wait for every check"
            );
            assert_eq!(
                text.contains("id-token"),
                *job == "publish-crates",
                "only publish-crates asks for an OIDC token"
            );
            assert!(
                code.iter()
                    .filter(|l| l.contains("ref_type"))
                    .all(|l| l.contains(ON_TAG)),
                "{job} tests ref_type without event_name: a dispatch on a tag would publish"
            );
            for step in steps(lines) {
                let step_text = step.join("\n");
                let gated_by_if = step.iter().any(|l| l.trim() == format!("if: {ON_TAG}"));
                let dry_run_unless_tag = step_text
                    .contains(&format!("DRY_RUN: ${{{{ !({ON_TAG}) && '--dry-run' || '' }}}}"))
                    && step
                        .iter()
                        .any(|l| l.contains("xtask publish") && l.contains("$DRY_RUN"));
                let real = [
                    "crates-io-auth-action",
                    "gh release",
                    "secrets.NPM_TOKEN",
                    "xtask publish crates",
                    "npm-stage",
                ];
                if real.iter().any(|p| step_text.contains(p)) {
                    assert!(
                        gated_by_if || dry_run_unless_tag,
                        "{job}: a step publishes without the tag-push gate:\n{step_text}"
                    );
                }
                if step_text.contains("secrets.NPM_TOKEN") {
                    assert!(
                        step.iter()
                            .any(|l| l.contains(&format!("${{{{ {ON_TAG} && secrets.NPM_TOKEN || '' }}}}"))),
                        "{job}: NPM_TOKEN reaches a run that isn't a tag push"
                    );
                }
            }
        }
    }

    /// The release lane calls nightly.yml with `record: false`, and every nightly step or job that
    /// writes (the bench-data pushes, the fuzz cache save, a history `--record`, the issues) is
    /// gated on that input.
    #[test]
    fn release_lane_should_call_the_nightly_lane_without_recording() {
        const RELEASE: &str = include_str!("../../../.github/workflows/release.yml");
        const NIGHTLY: &str = include_str!("../../../.github/workflows/nightly.yml");
        assert!(
            RELEASE.contains("      record: false\n"),
            "release.yml must pass `record: false`"
        );
        // Each step: the text from one `      - ` line to the next, without comment lines.
        let code = NIGHTLY
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        let steps: Vec<&str> = code.split("\n      - ").collect();
        let ungated: Vec<&str> = steps
            .iter()
            .filter(|step| {
                let writes = (step.contains("bench-history") && step.contains("mode: push"))
                    || step.contains("actions/cache/save")
                    || step.contains("fuzz cmin")
                    || step.contains("--record");
                // Gated in the expression, or in the shell (pdf-runtimes' `--record` branch).
                let gated = step.contains("env.RECORD == 'true'")
                    || step.contains("env.NIGHTLY_RECORD == 'true'")
                    || step.contains("if [ \"$RECORD\" = true ]");
                writes && !gated
            })
            .map(|step| step.lines().next().unwrap_or_default())
            .collect();
        assert_eq!(
            ungated,
            Vec::<&str>::new(),
            "nightly.yml steps that write without the record gate"
        );
        let record_env = NIGHTLY
            .lines()
            .filter(|l| l.trim_start().starts_with("RECORD: "))
            .collect::<Vec<_>>();
        assert!(
            record_env.len() == 3
                && record_env
                    .iter()
                    .all(|l| l.contains("toJSON(inputs.record) != 'false'")),
            "{record_env:?}"
        );
        assert!(NIGHTLY.contains("NIGHTLY_RECORD: ${{ toJSON(inputs.record) != 'false' }}"));
        let report = NIGHTLY.split("\n  report:\n").nth(1).unwrap_or_default();
        let report_if = report.split("runs-on:").next().unwrap_or_default();
        assert!(
            report_if.contains("toJSON(inputs.record) != 'false'"),
            "the report job must skip when not recording"
        );
        // Issues are filed only by the report job, so its gate covers every one (bench and
        // render-diff regressions included).
        // Each job: from one two-space-indented `<id>:` line to the next.
        let mut jobs: Vec<(String, String)> = Vec::new();
        for line in code.lines() {
            let job_line = line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':');
            match jobs.last_mut() {
                Some((_, body)) if !job_line => body.push_str(line),
                _ => jobs.push((line.trim().to_owned(), String::new())),
            }
        }
        let filing: Vec<&str> = jobs
            .iter()
            .filter(|(_, body)| body.contains("gh issue"))
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(filing, ["report:"], "only the report job may file issues");
        for output in ["needs.bench.outputs.regression", "needs.render-diff.outputs.regression"] {
            assert!(report_if.contains(output), "the report job must handle {output}");
        }
    }

    #[test]
    fn parse_args_should_take_a_run_id_and_a_commit() {
        let parse = |a: &[&str]| parse_args(&a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()).ok();
        assert_eq!(
            [
                parse(&[]),
                parse(&["--run", "123", "--commit", "v1.0.0"]),
                parse(&["--run", "x"])
            ],
            [
                Some(Args {
                    run: None,
                    commit: "HEAD".to_owned()
                }),
                Some(Args {
                    run: Some("123".to_owned()),
                    commit: "v1.0.0".to_owned()
                }),
                None
            ]
        );
    }

    #[test]
    fn notes_for_should_take_the_tags_section_without_its_heading() {
        let changelog = "# Changelog\n\n## 0.2.0\n\n### Minor Changes\n\n- b\n\n## 0.1.0\n\n- a\n";
        assert_eq!(notes_for(changelog, "v0.2.0").unwrap(), "### Minor Changes\n\n- b\n");
        assert!(notes_for(changelog, "v0.3.0").is_err());
        assert!(notes_for(changelog, "0.2.0").is_err());
    }
}
