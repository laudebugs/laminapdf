//! The desktop release's secret boundaries (docs/guidelines/desktop-release.md §1.9):
//! - `desktop-nightly` has no reviewer, so no job in it may name a secret that signs as the owner.
//! - The plan and the dry run, which pull requests start, read no secret and use no environment.
//! - Each build and publish job reads only its own environment's secrets.
//! - Windows signs through OIDC: only the release build's Windows legs read the Azure IDs.
//! - The Mac App Store package (todo D8c): its pull-request and dispatch `package` job reads no
//!   secret; only its `upload` job reads the store certificates and the profile, in `desktop-release`.
//! - The staging publish shares the update test's concurrency group (`desktop-update-e2e.yml`),
//!   so the two never write the staging channel at once.
//! - Only `archive-symbols` holds the symbols bucket's token, in `desktop-release` and only on a tag
//!   push, and the `dump_syms` it runs is doctor.rs's pin (todo D7d).

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
        if id == "plan" || id == "build-dry-run" || id == "build-app-store-package" {
            assert_eq!(environment(&lines), None, "{id}");
            // The message names the job only: CodeQL treats any secret's name in a log as a leak.
            assert!(secrets(&lines).is_empty(), "{id} reads a secret");
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
        let env = environment(&lines).unwrap_or_else(|| panic!("{id} reads a secret with no environment"));
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
fn the_staging_publish_should_share_the_update_tests_concurrency_group() {
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
    let update_test = repository_file(".github/workflows/desktop-update-e2e.yml").expect("the update test");
    assert!(
        update_test.contains("'desktop-staging-channel'"),
        "desktop-update-e2e.yml must take the same group"
    );
}

/// Windows signs through OIDC (runbook §1.5): only the release build reads the Azure IDs, only its
/// Windows legs get them, and no client secret exists for any workflow or action to name.
#[test]
fn only_the_release_builds_windows_legs_should_read_the_azure_ids() {
    let text = workflow();
    for (id, lines) in jobs(&text) {
        let azure: Vec<_> = lines.iter().filter(|l| l.contains("secrets.AZURE_")).collect();
        if id != "build-release" {
            assert!(azure.is_empty(), "{id} reads an Azure secret");
            continue;
        }
        assert_eq!(azure.len(), 3, "the client, tenant and subscription IDs");
        for line in azure {
            assert!(line.contains("matrix.os == 'windows' && secrets.AZURE_"), "{line}");
        }
        assert!(
            lines.iter().any(|l| l.trim() == "id-token: write"),
            "{id} needs GitHub's OIDC token"
        );
    }
    let action = repository_file(".github/actions/desktop-build/action.yml").expect("the build action");
    for text in [text.as_str(), action.as_str()] {
        assert!(
            !text.contains("AZURE_CLIENT_SECRET"),
            "Artifact Signing uses no client secret"
        );
    }
}

/// The Developer ID and the notarisation key reach the release build's macOS legs alone: every
/// other leg sees an empty value.
#[test]
fn the_apple_secrets_should_reach_only_the_macos_legs_of_the_release_build() {
    let text = workflow();
    let (_, lines) = jobs(&text)
        .into_iter()
        .find(|(id, _)| *id == "build-release")
        .expect("build-release");
    let apple: Vec<&&str> = lines
        .iter()
        .filter(|line| line.contains("secrets.APPLE_") || line.contains("secrets.KEYCHAIN_PASSWORD"))
        .collect();
    assert_eq!(apple.len(), 7, "the seven Apple secrets of runbook §1.3 and §1.4");
    for line in apple {
        assert!(line.contains("matrix.os == 'macos' && secrets."), "{line}");
    }
}

/// The Mac App Store's certificates and profile (runbook §8): only the store's upload job reads
/// them, beside the keychain and API key it shares with the direct download. No other job outside
/// the release build's macOS legs names an Apple secret.
#[test]
fn only_the_store_upload_should_read_the_store_certificates_and_the_profile() {
    let text = workflow();
    for (id, lines) in jobs(&text) {
        let read = secrets(&lines);
        let apple: Vec<_> = read
            .iter()
            .filter(|name| name.starts_with("APPLE_") || **name == "KEYCHAIN_PASSWORD")
            .collect();
        match id {
            "build-release" => {}
            "build-app-store" => {
                assert_eq!(environment(&lines), Some("desktop-release"));
                for name in [
                    "APPLE_DISTRIBUTION_CERTIFICATE",
                    "APPLE_DISTRIBUTION_CERTIFICATE_PASSWORD",
                    "APPLE_INSTALLER_CERTIFICATE",
                    "APPLE_INSTALLER_CERTIFICATE_PASSWORD",
                    "APPLE_APP_STORE_PROFILE",
                    "KEYCHAIN_PASSWORD",
                    "APPLE_API_KEY",
                    "APPLE_API_ISSUER",
                    "APPLE_API_KEY_P8",
                ] {
                    assert!(read.contains(&name), "the store upload reads {name}");
                }
                assert_eq!(apple.len(), 9, "{apple:?}");
                assert!(
                    lines
                        .iter()
                        .any(|l| l.contains("needs.plan.outputs.app-store == 'upload'")),
                    "it runs only when the plan says upload"
                );
            }
            _ => assert!(apple.is_empty(), "{id} reads {apple:?}"),
        }
    }
}

/// The package job and the upload job build the same package through one action; the dry run's
/// check is the upload's, with no secret.
#[test]
fn both_store_jobs_should_build_through_the_same_action() {
    let text = workflow();
    for id in ["build-app-store-package", "build-app-store"] {
        let (_, lines) = jobs(&text).into_iter().find(|(job, _)| *job == id).expect(id);
        let body = lines.join("\n");
        assert!(body.contains("uses: $/.github/actions/desktop-app-store"), "{id}");
        assert!(
            body.contains("runs-on: macos-15"),
            "{id}: an arm64 Mac builds both architectures"
        );
    }
}

/// The symbols bucket's secrets (todo D7d): only `archive-symbols` reads them.
const SYMBOLS_SECRETS: [&str; 2] = ["SYMBOLS_R2_TOKEN", "SYMBOLS_R2_ACCESS_KEY_ID"];

#[test]
fn the_symbols_should_be_archived_only_from_a_tag_push_in_desktop_release() {
    let text = workflow();
    let (_, lines) = jobs(&text)
        .into_iter()
        .find(|(id, _)| *id == "archive-symbols")
        .expect("archive-symbols");
    assert_eq!(environment(&lines), Some("desktop-release"));
    let body = lines.join("\n");
    assert!(
        body.contains("github.event_name == 'push' && github.ref_type == 'tag'"),
        "a staging dispatch or a dry run archives nothing"
    );
    let mut read = secrets(&lines);
    read.sort_unstable();
    assert_eq!(
        read,
        ["CLOUDFLARE_ACCOUNT_ID", "SYMBOLS_R2_ACCESS_KEY_ID", "SYMBOLS_R2_TOKEN"]
    );
    // A missing token is a notice, never a failure of the release.
    assert!(body.contains("::notice::") && body.contains("steps.token.outputs.set == 'true'"));
}

#[test]
fn no_other_job_should_read_the_symbols_token() {
    let text = workflow();
    for (id, lines) in jobs(&text) {
        if id != "archive-symbols" {
            let read = secrets(&lines);
            assert!(!SYMBOLS_SECRETS.iter().any(|s| read.contains(s)), "{id}");
        }
    }
}

#[test]
fn the_dump_syms_action_should_install_the_doctor_pin() {
    let action = repository_file(".github/actions/dump-syms/action.yml").expect("the dump_syms action");
    let pin = format!("DUMP_SYMS_VERSION: \"{}\"", crate::doctor::DUMP_SYMS_VERSION);
    assert!(
        action.contains(&pin),
        "the action pins dump_syms at doctor.rs's version"
    );
}

#[test]
fn the_secrets_reader_should_find_each_name() {
    let lines = [
        "      A: ${{ secrets.TAURI_KEY }}",
        "      B: ${{ x && secrets.LINUX_SIGNING_KEY || '' }}",
    ];
    assert_eq!(secrets(&lines), ["TAURI_KEY", "LINUX_SIGNING_KEY"]);
}

#[test]
fn the_plan_step_should_pass_the_tag_os_variable_and_no_other_job_should_read_it() {
    let text = workflow();
    let all = jobs(&text);
    let (_, plan) = all.iter().find(|(job, _)| *job == "plan").expect("plan");
    assert!(
        plan.join("\n").contains("DESKTOP_TAG_OS: ${{ vars.DESKTOP_TAG_OS }}"),
        "the plan step reads the repository variable"
    );
    for (id, lines) in &all {
        if *id != "plan" {
            assert!(!lines.join("\n").contains("DESKTOP_TAG_OS"), "{id}");
        }
    }
}

#[test]
fn the_publish_step_should_name_the_notes_page_when_the_site_has_one() {
    let action = repository_file(".github/actions/desktop-publish/action.yml").expect("the publish action");
    assert!(action.contains("apps/site/src/pages/desktop/releases/$VERSION.astro"));
    assert!(action.contains("NOTES=\"https://laminapdf.com/desktop/releases/$VERSION/\""));
    assert!(action.contains("--notes \"$NOTES\""));
}

#[test]
fn the_publish_step_should_check_the_notes_page_file_of_the_version() {
    let action = repository_file(".github/actions/desktop-publish/action.yml").expect("the publish action");
    assert!(
        action.contains("if [ -f \"apps/site/src/pages/desktop/releases/$VERSION.astro\" ]; then"),
        "the page's file name is the version, as the site's route is"
    );
}
