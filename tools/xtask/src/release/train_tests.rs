//! The owner installs these workflows from tools/release: never merge a partial installation.

use super::FULL_DISPATCH_TITLE;
use crate::workflows::{jobs, lines_of, needs};

const TRAIN: &str = include_str!("../../../release/release-train.yml");
const RELEASE: &str = include_str!("../../../release/release.yml");

#[test]
fn installed_release_workflows_should_match_the_reviewed_files() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, expected) in [("release-train.yml", TRAIN), ("release.yml", RELEASE)] {
        let path = root.join(".github/workflows").join(name);
        assert!(
            std::fs::read_to_string(&path).is_ok_and(|installed| installed == expected),
            "the owner must install tools/release/{name} as .github/workflows/{name} before merge"
        );
    }
}

#[test]
fn train_should_require_a_full_successful_main_nightly_and_honor_the_kill_switch() {
    let trigger = lines_of(TRAIN, "on", "workflow_run").join("\n");
    for condition in ["workflows: [Nightly]", "types: [completed]", "branches: [main]"] {
        assert!(trigger.contains(condition), "missing trigger {condition}");
    }
    let prepare = lines_of(TRAIN, "jobs", "prepare").join("\n");
    // The title nightly.yml gives a full dispatch (checked by release.rs's run-name test), so
    // renaming it on either side fails a test instead of silently stopping dispatched releases.
    let full_dispatch = format!("display_title == '{FULL_DISPATCH_TITLE}'");
    assert!(prepare.contains(&full_dispatch), "missing safeguard {full_dispatch}");
    for condition in [
        "vars.RELEASE_TRAIN != 'off'",
        "head_repository.full_name == github.repository",
        "head_branch == 'main'",
        "conclusion == 'success'",
        "event == 'schedule'",
        "event == 'workflow_dispatch'",
        "ref: ${{ github.event.workflow_run.head_sha }}",
        "persist-credentials: false",
        "fetch-depth: 0",
        "NIGHTLY_RUN: ${{ github.event.workflow_run.id }}",
        "cargo xtask release prepare --run \"$NIGHTLY_RUN\"",
    ] {
        assert!(prepare.contains(condition), "missing safeguard {condition}");
    }
    assert!(TRAIN.contains("group: release-train\n  cancel-in-progress: false"));
}

#[test]
fn only_a_prepared_release_should_push_both_refs_with_the_app_token() {
    let prepare = lines_of(TRAIN, "jobs", "prepare").join("\n");
    let after = prepare.split("name: Create the release bot's token").nth(1).unwrap();
    assert_eq!(after.matches("if: steps.version.outputs.ready == 'true'").count(), 2);
    for part in [
        "actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510",
        "GH_TOKEN: ${{ steps.bot.outputs.token }}",
        "push --atomic origin \"HEAD:refs/heads/$BRANCH\" \"refs/tags/$TAG:refs/tags/$TAG\"",
    ] {
        assert!(after.contains(part), "missing push safeguard {part}");
    }
    assert!(!after.contains("--force"));
}

#[test]
fn docs_should_wait_for_publishing_and_registry_checks_and_require_credentials() {
    let ready = lines_of(RELEASE, "jobs", "docs-ready");
    for dependency in ["publish-crates", "publish-npm", "github-release", "verify-registry"] {
        assert!(needs(RELEASE, "docs-ready").contains(&dependency));
        assert!(
            ready
                .join("\n")
                .contains(&format!("needs.{dependency}.result == 'success'"))
        );
    }
    let text = ready.join("\n");
    assert!(text.contains("[ -z \"$CLOUDFLARE_API_TOKEN\" ] || [ -z \"$CLOUDFLARE_ACCOUNT_ID\" ]"));
    let docs = lines_of(RELEASE, "jobs", "docs");
    assert_eq!(needs(RELEASE, "docs"), vec!["guard", "docs-ready"]);
    assert!(docs.join("\n").contains("uses: ./.github/workflows/docs.yml"));
    assert!(
        docs.join("\n").contains("pull-requests: write"),
        "the called workflow declares a skipped PR-comment job"
    );
    assert!(docs.join("\n").contains("tag: ${{ needs.guard.outputs.ref }}"));
}

#[test]
fn version_pr_should_merge_only_the_published_commit_after_the_docs_deploy() {
    let pr = lines_of(RELEASE, "jobs", "version-pr");
    assert_eq!(needs(RELEASE, "version-pr"), vec!["guard", "docs"]);
    let text = pr.join("\n");
    for part in [
        "needs.docs.result == 'success'",
        "GH_TOKEN: ${{ steps.bot.outputs.token }}",
        "COMMIT: ${{ github.sha }}",
        "if [ -z \"$remote\" ]",
        "if [ \"${remote%%[[:space:]]*}\" != \"$COMMIT\" ]",
        "gh pr list --head \"$branch\" --base main --state open",
        "gh pr merge \"$pr\" --auto --squash --match-head-commit \"$COMMIT\"",
    ] {
        assert!(text.contains(part), "missing merge safeguard {part}");
    }
    for (name, lines) in jobs(RELEASE) {
        if ["docs-ready", "docs", "version-pr"].contains(&name) {
            assert!(
                lines
                    .join("\n")
                    .contains("github.event_name == 'push' && github.ref_type == 'tag'")
            );
        }
    }
}
