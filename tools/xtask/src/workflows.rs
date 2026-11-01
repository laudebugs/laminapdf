//! Tests that keep the two required checks complete (todo X7): every job of ci.yml is in
//! `fast-gate`'s `needs`, and every job of full.yml is in `gate`'s. A job left out of them
//! would not block a merge when it fails.
//!
//! The parsing is line based, not a YAML parser, and assumes the workflows' layout: jobs are
//! two-space-indented keys under `jobs:`, and a gate lists its `needs` on one line. A gate whose
//! `needs` is written any other way reads as empty, so every job is reported missing. The
//! module is compiled only for tests (`main.rs`).

/// The `<id>` of a two-space-indented `<id>:` line (a trailing comment allowed), else `None`.
pub(crate) fn job_id(line: &str) -> Option<&str> {
    if !line.starts_with("  ") || line.starts_with("   ") || line.trim_start().starts_with('#') {
        return None;
    }
    let key = line.split(" #").next().unwrap_or_default().trim_end();
    key.strip_suffix(':').map(str::trim)
}

/// The job ids of a workflow, in order.
fn job_ids(workflow: &str) -> Vec<&str> {
    let jobs = workflow.split("\njobs:\n").nth(1).unwrap_or_default();
    jobs.lines().filter_map(job_id).collect()
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

/// The ids in `job`'s one-line `needs: [a, b]`, read only from that job's own lines.
fn needs<'a>(workflow: &'a str, job: &str) -> Vec<&'a str> {
    let jobs = workflow.split("\njobs:\n").nth(1).unwrap_or_default();
    jobs.lines()
        .skip_while(|l| job_id(l) != Some(job))
        .skip(1)
        .take_while(|l| job_id(l).is_none())
        .find_map(|l| l.trim().strip_prefix("needs: ["))
        .and_then(|l| l.strip_suffix(']'))
        .map(|l| l.split(',').map(str::trim).collect())
        .unwrap_or_default()
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

    #[test]
    fn fast_gate_should_need_every_ci_job() {
        assert_eq!(missing_from(CI, "fast-gate"), Vec::<&str>::new());
    }

    #[test]
    fn gate_should_need_every_full_lane_job() {
        assert_eq!(missing_from(FULL, "gate"), Vec::<&str>::new());
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
