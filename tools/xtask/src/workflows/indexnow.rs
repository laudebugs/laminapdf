//! IndexNow pings only follow production deploys (todo W3e). Of every workflow, only site.yml's and
//! docs.yml's `production` jobs run `cargo xtask indexnow submit`; each records the live sitemap
//! before its deploy and submits after it, in a step that runs only when every step before it
//! succeeded (no `always()`, `failure()` or `cancelled()`), and docs.yml's only on a real deploy,
//! never a dry run. The key file each names is the one its site serves from `public/`.

use super::{jobs, repository_file, workflow_files};

/// The jobs allowed to ping, as (workflow, job), in the order [`workflow_files`] lists them.
const PRODUCTION: [(&str, &str); 2] = [
    (".github/workflows/docs.yml", "production"),
    (".github/workflows/site.yml", "production"),
];

/// The step around `at` among `lines`: from its `- ` line to the next step at that indent.
fn step_around<'a>(lines: &[&'a str], at: usize) -> Vec<&'a str> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let start = (0..=at)
        .rev()
        .find(|&i| lines[i].trim_start().starts_with("- "))
        .unwrap_or(at);
    let own = indent(lines[start]);
    let end = (start + 1..lines.len())
        .find(|&i| {
            let l = lines[i];
            !l.trim().is_empty() && (indent(l) < own || (indent(l) == own && l.trim_start().starts_with("- ")))
        })
        .unwrap_or(lines.len());
    lines[start..end].to_vec()
}

/// The `if:` of a step, empty when it has none.
fn condition(step: &[&str]) -> String {
    step.iter()
        .find_map(|l| l.trim().strip_prefix("if:"))
        .map(|c| c.trim().to_owned())
        .unwrap_or_default()
}

#[test]
fn only_the_production_deploys_should_ping_indexnow() {
    let mut pinging = Vec::new();
    for file in workflow_files() {
        let text = repository_file(&file).unwrap_or_default();
        for (job, lines) in jobs(&text) {
            if lines.iter().any(|l| l.contains("cargo xtask indexnow submit")) {
                pinging.push((file.clone(), job.to_owned()));
            }
        }
    }
    let expected: Vec<(String, String)> = PRODUCTION
        .iter()
        .map(|(f, j)| ((*f).to_owned(), (*j).to_owned()))
        .collect();
    assert_eq!(pinging, expected);
}

#[test]
fn each_production_deploy_should_snapshot_before_and_submit_after_a_successful_deploy() {
    for (file, job) in PRODUCTION {
        let text = repository_file(file).unwrap_or_else(|| panic!("no {file}"));
        let lines = jobs(&text)
            .into_iter()
            .find(|(id, _)| *id == job)
            .map_or_else(|| panic!("{file} has no {job} job"), |(_, lines)| lines);
        let at = |want: &str| lines.iter().position(|l| l.contains(want));
        let snapshot = at("cargo xtask indexnow snapshot").unwrap_or_else(|| panic!("{file}: no snapshot"));
        let deploy = lines
            .iter()
            .position(|l| {
                let l = l.trim();
                let l = l.strip_prefix("run: ").unwrap_or(l);
                l.starts_with("pnpm exec wrangler deploy") && !l.ends_with("--dry-run")
            })
            .unwrap_or_else(|| panic!("{file}: no deploy"));
        let submit = at("cargo xtask indexnow submit").unwrap_or_else(|| panic!("{file}: no submit"));
        assert!(
            snapshot < deploy && deploy < submit,
            "{file}: snapshot, deploy, submit out of order"
        );
        let deploy_step = step_around(&lines, deploy);
        assert!(
            !deploy_step.iter().any(|l| l.trim().starts_with("continue-on-error")),
            "{file}: the deploy may fail and still be followed by a ping"
        );
        let submit_if = condition(&step_around(&lines, submit));
        for status in ["always()", "failure()", "cancelled()"] {
            assert!(!submit_if.contains(status), "{file}: the ping runs on {status}");
        }
        if file.ends_with("docs.yml") {
            assert_eq!(
                submit_if, "steps.mode.outputs.mode == 'deploy'",
                "{file}: a dry run would ping"
            );
            assert_eq!(
                condition(&step_around(&lines, snapshot)),
                "steps.mode.outputs.mode == 'deploy'",
                "{file}: a dry run would fetch the live sitemap"
            );
        }
    }
}

/// The key file a submission names is the one the site it deploys serves from `public/`.
#[test]
fn each_submission_should_name_its_own_sites_key_file() {
    for (file, site) in [
        (".github/workflows/site.yml", "apps/site"),
        (".github/workflows/docs.yml", "apps/docs"),
    ] {
        let text = repository_file(file).unwrap_or_default();
        let key_file = text
            .split_whitespace()
            .skip_while(|w| *w != "--key-file")
            .nth(1)
            .unwrap_or_else(|| panic!("{file} names no --key-file"));
        assert!(key_file.starts_with(&format!("{site}/public/")), "{file}: {key_file}");
        assert!(repository_file(key_file).is_some(), "{file}: {key_file} doesn't exist");
    }
}
