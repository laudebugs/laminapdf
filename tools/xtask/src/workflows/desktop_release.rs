//! The desktop release's secret boundaries (todo D6b, docs/guidelines/desktop-release.md §1.9):
//! `desktop-nightly` has no reviewer, so no job in it may name a secret that signs as the owner;
//! the plan and the dry run, which pull requests start, read no secret and use no environment;
//! each build and publish job reads only its own environment's secrets; and the staging publish
//! shares D5c's concurrency group, so the two never write the staging channel at once.

use super::{jobs, repository_file};

const WORKFLOW: &str = ".github/workflows/desktop-release.yml";

/// The secrets only `desktop-release` holds: Apple's, Azure's and the Linux GPG key.
const OWNER_SECRETS: [&str; 3] = ["APPLE_", "AZURE_", "LINUX_SIGNING_"];

fn workflow() -> String {
    repository_file(WORKFLOW).expect("the desktop release workflow")
}

/// The `environment:` a job names, if any.
fn environment<'a>(lines: &[&'a str]) -> Option<&'a str> {
    lines
        .iter()
        .find_map(|line| line.strip_prefix("    environment:"))
        .map(str::trim)
}

/// The secrets a job's lines name, as `secrets.<NAME>`.
fn secrets<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .flat_map(|line| {
            line.match_indices("secrets.")
                .map(move |(at, _)| &line[at + "secrets.".len()..])
        })
        .map(|rest| {
            rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn no_nightly_job_should_name_a_secret_that_signs_as_the_owner() {
    let text = workflow();
    let nightly: Vec<_> = jobs(&text)
        .into_iter()
        .filter(|(_, lines)| environment(lines) == Some("desktop-nightly"))
        .collect();
    assert!(nightly.len() >= 2, "the nightly build and publish jobs");
    for (id, lines) in nightly {
        let body = lines.join("\n");
        for prefix in OWNER_SECRETS {
            assert!(
                !body.contains(prefix),
                "{id} runs in desktop-nightly and names {prefix}*"
            );
        }
    }
}

#[test]
fn the_plan_and_the_dry_run_should_read_no_secret_and_use_no_environment() {
    let text = workflow();
    for (id, lines) in jobs(&text) {
        if id == "plan" || id == "build-dry-run" {
            assert_eq!(environment(&lines), None, "{id}");
            assert!(secrets(&lines).is_empty(), "{id} reads {:?}", secrets(&lines));
        }
    }
}

#[test]
fn every_job_that_reads_a_secret_should_name_its_environment() {
    let text = workflow();
    for (id, lines) in jobs(&text) {
        let read = secrets(&lines);
        if read.is_empty() {
            continue;
        }
        let env = environment(&lines).unwrap_or_else(|| panic!("{id} reads {read:?} with no environment"));
        assert!(env == "desktop-nightly" || env == "desktop-release", "{id}: {env}");
        let condition = lines
            .iter()
            .take_while(|l| !l.starts_with("    environment:"))
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            condition.contains(&format!("needs.plan.outputs.environment == '{env}'")),
            "{id} runs in {env} only when the plan says so"
        );
    }
}

#[test]
fn the_staging_publish_should_share_d5cs_concurrency_group() {
    let text = workflow();
    let publish = jobs(&text)
        .into_iter()
        .find(|(id, _)| *id == "publish-release")
        .expect("publish-release");
    assert!(
        publish
            .1
            .iter()
            .any(|l| l.contains("'desktop-staging-channel'") && l.contains("group:"))
    );
    assert!(publish.1.iter().any(|l| l.trim() == "cancel-in-progress: false"));
}

#[test]
fn the_secrets_reader_should_find_each_name() {
    let lines = [
        "      A: ${{ secrets.TAURI_KEY }}",
        "      B: ${{ x && secrets.LINUX_SIGNING_KEY || '' }}",
    ];
    assert_eq!(secrets(&lines), ["TAURI_KEY", "LINUX_SIGNING_KEY"]);
}
