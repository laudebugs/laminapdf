//! Tests of the workflows' job graphs. The two required checks stay complete (todo X7): every
//! job of ci.yml is in `fast-gate`'s `needs`, and every job of full.yml is in `gate`'s, since a
//! job left out of them would not block a merge when it fails. In nightly.yml, `report` needs
//! every job (else that job's failure opens no issue) and `bench` needs every job that doesn't
//! depend on it (why: the comment on nightly.yml's `bench` job).
//!
//! The parsing is line based, not a YAML parser, and assumes the workflows' layout: jobs are
//! two-space-indented keys under `jobs:`, and a job lists its `needs` on one line, as `[a, b]`
//! or a single id. A gate whose `needs` is written any other way reads as empty, so every job is
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
    let body = workflow.split("\njobs:\n").nth(1).unwrap_or_default();
    let mut jobs: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in body.lines() {
        if let Some(id) = job_id(line) {
            jobs.push((id, Vec::new()));
        } else if let Some((_, lines)) = jobs.last_mut() {
            lines.push(line);
        }
    }
    jobs
}

/// The job ids of a workflow, in order.
fn job_ids(workflow: &str) -> Vec<&str> {
    jobs(workflow).into_iter().map(|(id, _)| id).collect()
}

/// The ids in `job`'s one-line `needs: [a, b]` or `needs: a`, read only from that job's own lines.
fn needs<'a>(workflow: &'a str, job: &str) -> Vec<&'a str> {
    jobs(workflow)
        .into_iter()
        .find(|(id, _)| *id == job)
        .and_then(|(_, lines)| lines.into_iter().find_map(|l| l.trim().strip_prefix("needs: ")))
        .map(|l| {
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

    const CI: &str = include_str!("../../../.github/workflows/ci.yml");
    const FULL: &str = include_str!("../../../.github/workflows/full.yml");
    const NIGHTLY: &str = include_str!("../../../.github/workflows/nightly.yml");

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

    #[test]
    fn needs_should_read_a_single_job_id() {
        let workflow = "on:\n  push:\njobs:\n  a:\n    run: x\n  b:\n    needs: a\n";
        assert_eq!(needs(workflow, "b"), ["a"]);
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
