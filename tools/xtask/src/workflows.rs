//! Tests of the workflows' job graphs. The two required checks stay complete (todo X7): every
//! job of ci.yml is in `fast-gate`'s `needs`, and every job of full.yml is in `gate`'s, since a
//! job left out of them would not block a merge when it fails. In nightly.yml, `report` needs
//! every job (else that job's failure opens no issue) and `bench` needs every job that doesn't
//! depend on it (why: the comment on nightly.yml's `bench` job). site.yml's production job
//! regenerates the website's roadmap before every build, and runs on every push to main and on
//! a schedule, so the site follows the board without a refresh PR. docs.yml deploys main's docs to
//! next.docs.laminapdf.com and only a release tag's to docs.laminapdf.com (todo P4).
//!
//! The parsing is line based, not a YAML parser, and assumes the workflows' layout: jobs and
//! triggers are two-space-indented keys under the top-level `jobs:` and `on:`, and a job lists its
//! `needs` on one line, as `[a, b]` or a single id, a trailing comment allowed. A gate whose `needs` is written any other way reads as empty, so every job is
//! reported missing. The module is compiled only for tests (`main.rs`).

/// The `<id>` of a two-space-indented `<id>:` line (a trailing comment allowed), else `None`.
pub(crate) fn job_id(line: &str) -> Option<&str> {
    if !line.starts_with("  ") || line.starts_with("   ") || line.trim_start().starts_with('#') {
        return None;
    }
    let key = line.split(" #").next().unwrap_or_default().trim_end();
    key.strip_suffix(':').map(str::trim)
}

/// Every job of `workflow` with its own lines (the `<id>:` line left out), in order.
pub(crate) fn jobs(workflow: &str) -> Vec<(&str, Vec<&str>)> {
    keys_under(workflow, "jobs")
}

/// The two-space-indented keys under `workflow`'s top-level `<section>:` line (a trailing comment
/// allowed), each with its own lines, in order. The section ends at the next top-level key.
fn keys_under<'a>(workflow: &'a str, section: &str) -> Vec<(&'a str, Vec<&'a str>)> {
    let top_level = |line: &str| !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#');
    let header = format!("{section}:");
    let body = workflow
        .lines()
        .skip_while(|line| !(top_level(line) && line.split(" #").next().unwrap_or_default().trim_end() == header))
        .skip(1);
    let mut keys: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in body {
        if top_level(line) {
            break;
        }
        if let Some(id) = job_id(line) {
            keys.push((id, Vec::new()));
        } else if let Some((_, lines)) = keys.last_mut() {
            lines.push(line);
        }
    }
    keys
}

/// The lines of `key` under `workflow`'s top-level `section` (empty when there is none).
pub(crate) fn lines_of<'a>(workflow: &'a str, section: &str, key: &str) -> Vec<&'a str> {
    keys_under(workflow, section)
        .into_iter()
        .find(|(id, _)| *id == key)
        .map(|(_, lines)| lines)
        .unwrap_or_default()
}

/// The job ids of a workflow, in order.
fn job_ids(workflow: &str) -> Vec<&str> {
    jobs(workflow).into_iter().map(|(id, _)| id).collect()
}

/// The ids in `job`'s one-line `needs: [a, b]` or `needs: a`, read only from that job's own lines.
pub(crate) fn needs<'a>(workflow: &'a str, job: &str) -> Vec<&'a str> {
    jobs(workflow)
        .into_iter()
        .find(|(id, _)| *id == job)
        .and_then(|(_, lines)| lines.into_iter().find_map(|l| l.trim().strip_prefix("needs: ")))
        .map(|l| {
            let l = l.split(" #").next().unwrap_or_default().trim_end();
            let list = l.strip_prefix('[').and_then(|l| l.strip_suffix(']')).unwrap_or(l);
            list.split(',').map(str::trim).collect()
        })
        .unwrap_or_default()
}

/// The jobs of `workflow` that need `job`, directly or through another job.
fn dependents<'a>(workflow: &'a str, job: &str) -> Vec<&'a str> {
    let ids = job_ids(workflow);
    let mut found: Vec<&str> = Vec::new();
    let mut grew = true;
    while grew {
        grew = false;
        for id in &ids {
            let after = needs(workflow, id).iter().any(|n| *n == job || found.contains(n));
            if *id != job && !found.contains(id) && after {
                found.push(id);
                grew = true;
            }
        }
    }
    found
}

/// The jobs of `workflow` missing from `gate`'s `needs`.
fn missing_from<'a>(workflow: &'a str, gate: &str) -> Vec<&'a str> {
    let needed = needs(workflow, gate);
    job_ids(workflow)
        .into_iter()
        .filter(|id| *id != gate && !needed.contains(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    const CI: &str = include_str!("../../../.github/workflows/ci.yml");
    const FULL: &str = include_str!("../../../.github/workflows/full.yml");
    const NIGHTLY: &str = include_str!("../../../.github/workflows/nightly.yml");
    const SITE: &str = include_str!("../../../.github/workflows/site.yml");
    const DOCS: &str = include_str!("../../../.github/workflows/docs.yml");

    fn yaml_files(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("workflow directory must be readable") {
            let path = entry.expect("workflow entry must be readable").path();
            if path.is_dir() {
                yaml_files(&path, files);
            } else if matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("yml" | "yaml")
            ) {
                files.push(path);
            }
        }
    }

    #[test]
    fn external_actions_should_be_owner_controlled_and_commit_pinned() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("xtask must be under the repository root");
        let mut files = Vec::new();
        yaml_files(&root.join(".github"), &mut files);
        yaml_files(&root.join("tools/release"), &mut files);
        let mut checked = 0;

        for path in files {
            let workflow = fs::read_to_string(&path).expect("workflow must be readable");
            for (line_number, line) in workflow.lines().enumerate() {
                let line = line.trim_start();
                let action = line
                    .strip_prefix("- uses: ")
                    .or_else(|| line.strip_prefix("uses: "))
                    .and_then(|uses| uses.split_whitespace().next());
                let Some(action) = action else { continue };
                if action.starts_with("./") {
                    continue;
                }

                checked += 1;
                let location = format!("{}:{}", path.display(), line_number + 1);
                let (repository, revision) = action
                    .split_once('@')
                    .unwrap_or_else(|| panic!("{location}: external action is not pinned: {action}"));
                assert!(
                    repository.starts_with("laudebugs/"),
                    "{location}: external action is not owned by laudebugs: {action}"
                );
                assert!(
                    revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "{location}: external action is not pinned to a full commit SHA: {action}"
                );
            }
        }

        assert!(checked > 0, "no external action references were checked");
    }

    #[test]
    fn fast_gate_should_need_every_ci_job() {
        assert_eq!(missing_from(CI, "fast-gate"), Vec::<&str>::new());
    }

    #[test]
    fn gate_should_need_every_full_lane_job() {
        assert_eq!(missing_from(FULL, "gate"), Vec::<&str>::new());
    }

    #[test]
    fn nightly_report_should_need_every_nightly_job() {
        assert_eq!(missing_from(NIGHTLY, "report"), Vec::<&str>::new());
    }

    /// `bench` needs every job but those that need it (`pdf-runtimes`, `report`).
    #[test]
    fn nightly_bench_should_need_every_job_that_does_not_depend_on_it() {
        let after = dependents(NIGHTLY, "bench");
        let missing: Vec<&str> = missing_from(NIGHTLY, "bench")
            .into_iter()
            .filter(|id| !after.contains(id))
            .collect();
        assert_eq!(missing, Vec::<&str>::new());
    }

    /// A job that hits its `timeout-minutes` ends `cancelled` but still fails the run, so
    /// `report` must count `cancelled` results too, and skip only a run someone cancelled.
    #[test]
    fn nightly_report_should_treat_a_timed_out_job_as_a_failure() {
        let lines = jobs(NIGHTLY)
            .into_iter()
            .find(|(id, _)| *id == "report")
            .map(|(_, lines)| lines)
            .unwrap_or_default();
        assert!(!lines.is_empty(), "nightly.yml has no `report` job");
        let condition: String = lines
            .iter()
            .skip_while(|l| !l.trim_start().starts_with("if:"))
            .take_while(|l| !l.trim_start().starts_with("runs-on:"))
            .flat_map(|l| l.split_whitespace())
            .collect::<Vec<_>>()
            .join(" ");
        for part in [
            "!cancelled()",
            "contains(needs.*.result, 'failure')",
            "contains(needs.*.result, 'cancelled')",
        ] {
            assert!(condition.contains(part), "report's `if` lacks {part}: {condition}");
        }
    }

    /// The roadmap (`cargo xtask progress`) is regenerated by each production build, so the
    /// site shows the board as it is on main. It reads main's history (`git log`) and the open
    /// pull requests, which claim the todos in progress.
    #[test]
    fn site_production_should_refresh_the_roadmap_before_building() {
        let lines = lines_of(SITE, "jobs", "production");
        assert!(!lines.is_empty(), "site.yml has no `production` job");
        let at = |want: &str| lines.iter().position(|l| l.trim() == want);
        let refresh = at("run: cargo xtask progress");
        let build = at("run: pnpm --filter @laminapdf/site build");
        assert!(refresh.is_some(), "production never runs `cargo xtask progress`");
        assert!(build.is_some(), "production never builds the site");
        assert!(
            refresh < build,
            "production builds the site before refreshing the roadmap"
        );
        for want in ["fetch-depth: 0", "pull-requests: read", "GH_TOKEN: ${{ github.token }}"] {
            assert!(at(want).is_some(), "production lacks `{want}`");
        }
    }

    /// Any merge can move a todo, whatever files it touches, so every push to main deploys; a
    /// schedule picks up the pull requests opened since, whose todos are now in progress.
    #[test]
    fn site_should_deploy_on_every_push_to_main_and_on_a_schedule() {
        let push = lines_of(SITE, "on", "push");
        assert!(!push.is_empty(), "site.yml doesn't run on push");
        let paths: Vec<&str> = push.iter().copied().filter(|l| l.trim().starts_with("paths")).collect();
        assert_eq!(paths, Vec::<&str>::new(), "site.yml's push trigger is filtered by path");
        assert!(lines_of(SITE, "on", "schedule").iter().any(|l| l.contains("cron:")));
    }

    /// main's docs go to next.docs.laminapdf.com as unreleased (no `LAMINA_PDF_ENV=production`, so
    /// noindex and the notice); docs.laminapdf.com gets only a release tag's, built as production,
    /// when the release lane calls with the tag (or by hand with one). Only that build is indexed.
    #[test]
    fn docs_should_deploy_main_to_next_and_only_release_tags_to_production() {
        let next = lines_of(DOCS, "jobs", "next");
        let production = lines_of(DOCS, "jobs", "production");
        assert!(
            !next.is_empty() && !production.is_empty(),
            "docs.yml lacks a `next` or `production` job"
        );
        let has = |lines: &[&str], want: &str| lines.iter().any(|l| l.trim() == want);
        // A deploy, and a dry run of the same Worker for the `dry-run` input.
        let deploys = |lines: &[&str], args: &str| {
            let command = format!("pnpm exec wrangler deploy {args} ");
            lines.iter().any(|l| {
                let l = l.trim();
                l.starts_with(&command) && !l.ends_with("--dry-run")
            }) && has(lines, &format!("pnpm exec wrangler deploy {args} --dry-run"))
        };
        assert!(
            has(
                &next,
                "if: ${{ github.ref == 'refs/heads/main' && (github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && inputs.tag == '')) }}"
            ),
            "next must run on pushes to main"
        );
        assert!(deploys(&next, "--env next"), "next must deploy the next Worker");
        assert!(
            has(&production, "if: ${{ inputs.tag != '' }}"),
            "production must run only with a tag"
        );
        assert!(
            has(&production, "ref: refs/tags/${{ inputs.tag }}"),
            "production must build the tag"
        );
        assert!(
            deploys(&production, "--env=\"\""),
            "production must deploy the top-level Worker"
        );
        let production_builds: Vec<&str> = jobs(DOCS)
            .into_iter()
            .filter(|(_, lines)| has(lines, "LAMINA_PDF_ENV: production"))
            .map(|(id, _)| id)
            .collect();
        assert_eq!(
            production_builds,
            ["production"],
            "only the release build may be indexed"
        );
        let call = lines_of(DOCS, "on", "workflow_call");
        assert!(
            has(&call, "tag:") && has(&call, "dry-run:"),
            "the release lane can't call docs.yml with a tag, or as a dry run"
        );
        assert!(
            has(&lines_of(DOCS, "on", "workflow_dispatch"), "dry-run:"),
            "a manual run can't be a dry run"
        );
        assert!(
            !DOCS.contains("X-Robots-Tag"),
            "the build marks noindex; the workflow shouldn't"
        );
    }

    #[test]
    fn keys_under_should_stop_at_the_next_top_level_key() {
        let workflow = "name: x\non: # triggers\n  push:\n    branches: [main]\n  # a comment\n  schedule:\n    - cron: \"0 1 * * *\"\npermissions:\n  contents: read\n";
        let ids: Vec<&str> = keys_under(workflow, "on").into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["push", "schedule"]);
        assert_eq!(
            lines_of(workflow, "on", "push"),
            ["    branches: [main]", "  # a comment"]
        );
    }

    #[test]
    fn needs_should_read_a_single_job_id() {
        let workflow = "on:\n  push:\njobs:\n  a:\n    run: x\n  b:\n    needs: a\n";
        assert_eq!(needs(workflow, "b"), ["a"]);
    }

    #[test]
    fn needs_should_ignore_a_trailing_comment() {
        let workflow =
            "on:\n  push:\njobs:\n  a:\n    run: x\n  b:\n    needs: a # why\n  c:\n    needs: [a, b] # why\n";
        assert_eq!(needs(workflow, "b"), ["a"]);
        assert_eq!(needs(workflow, "c"), ["a", "b"]);
    }

    #[test]
    fn dependents_should_follow_needs_through_other_jobs() {
        let workflow =
            "on:\n  push:\njobs:\n  a:\n    run: x\n  b:\n    needs: a\n  c:\n    needs: [b]\n  d:\n    run: y\n";
        assert_eq!(dependents(workflow, "a"), ["b", "c"]);
    }

    #[test]
    fn missing_from_should_report_a_job_left_out_of_the_gate() {
        let workflow = "on:\n  push:\njobs:\n  a:\n    run: x\n  b:\n    run: y\n  g:\n    needs: [a]\n";
        assert_eq!(missing_from(workflow, "g"), ["b"]);
    }

    #[test]
    fn job_ids_should_read_a_job_with_a_trailing_comment_and_skip_comment_lines() {
        let workflow = "on:\n  push:\njobs:\n  # note:\n  a: # first\n    run: x\n";
        assert_eq!(job_ids(workflow), ["a"]);
    }

    #[test]
    fn needs_should_not_read_the_next_jobs_needs() {
        let workflow = "on:\n  push:\njobs:\n  a:\n    run: w\n  g:\n    run: x\n  b:\n    needs: [a]\n";
        assert_eq!(needs(workflow, "g"), Vec::<&str>::new());
    }

    #[test]
    fn job_ids_should_ignore_keys_outside_jobs() {
        let workflow = "on:\n  push:\n  pull_request:\njobs:\n  a:\n    steps:\n";
        assert_eq!(job_ids(workflow), ["a"]);
    }
}
