//! Tests that keep the two required checks complete (todo X7): every job of ci.yml is in
//! `fast-gate`'s `needs`, and every job of full.yml is in `gate`'s. A job left out of them
//! would not block a merge when it fails.

/// The job ids of a workflow: two-space-indented `<id>:` lines under `jobs:`.
fn job_ids(workflow: &str) -> Vec<&str> {
    let jobs = workflow.split("\njobs:\n").nth(1).unwrap_or_default();
    jobs.lines()
        .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.trim_end().ends_with(':'))
        .map(|l| l.trim().trim_end_matches(':'))
        .collect()
}

/// The ids in `job`'s one-line `needs: [a, b]`.
fn needs<'a>(workflow: &'a str, job: &str) -> Vec<&'a str> {
    let body = workflow.split(&format!("\n  {job}:\n")).nth(1).unwrap_or_default();
    body.lines()
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
    fn job_ids_should_ignore_keys_outside_jobs() {
        let workflow = "on:\n  push:\n  pull_request:\njobs:\n  a:\n    steps:\n";
        assert_eq!(job_ids(workflow), ["a"]);
    }
}
