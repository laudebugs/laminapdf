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
        "ref: ${{ steps.nightly.outputs.sha }}",
        "persist-credentials: false",
        "fetch-depth: 0",
        "NIGHTLY_RUN: ${{ steps.nightly.outputs.id }}",
        "cargo xtask release prepare --run \"$NIGHTLY_RUN\"",
    ] {
        assert!(prepare.contains(condition), "missing safeguard {condition}");
    }
    assert!(TRAIN.contains("group: release-train\n  cancel-in-progress: false"));
}

#[test]
fn manual_train_should_require_a_run_id_and_run_only_on_main() {
    let trigger = lines_of(TRAIN, "on", "workflow_dispatch").join("\n");
    for part in ["nightly-run-id:", "required: true", "type: string"] {
        assert!(trigger.contains(part), "missing manual input {part}");
    }
    let prepare = lines_of(TRAIN, "jobs", "prepare").join("\n");
    assert!(prepare.contains("github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'"));
    assert!(prepare.contains("NIGHTLY_RUN: ${{ github.event.workflow_run.id || inputs.nightly-run-id }}"));
}

#[test]
fn selected_nightly_should_be_validated_before_its_code_is_checked_out() {
    let prepare = lines_of(TRAIN, "jobs", "prepare").join("\n");
    let validate = prepare.find("name: Select the completed full nightly").unwrap();
    let checkout = prepare.find("uses: actions/checkout@").unwrap();
    assert!(validate < checkout);
    let selector = prepare.get(validate..checkout).unwrap();
    let full_dispatch = format!(".displayTitle == \"{FULL_DISPATCH_TITLE}\"");
    assert!(selector.contains(&full_dispatch));
    for part in [
        "^[0-9]+$",
        ".workflowName == \"Nightly\"",
        ".headBranch == \"main\"",
        ".status == \"completed\"",
        ".conclusion == \"success\"",
        ".event == \"schedule\"",
        ".event == \"workflow_dispatch\"",
        "^[0-9a-f]{40}$",
        "echo \"id=$NIGHTLY_RUN\"",
        "echo \"sha=$nightly_sha\"",
    ] {
        assert!(selector.contains(part), "missing selected-run validation {part}");
    }
}

#[test]
fn train_should_recheck_the_latest_nightly_before_pushing_a_prepared_release() {
    let prepare = lines_of(TRAIN, "jobs", "prepare").join("\n");
    let guard = prepare.find("run: cargo xtask release-guard --commit HEAD").unwrap();
    let token = prepare.find("name: Create the release bot's token").unwrap();
    assert!(guard < token);
    let before = prepare.get(..guard).unwrap();
    let step = before.rsplit("      - name:").next().unwrap();
    assert!(step.contains("if: steps.version.outputs.ready == 'true'"));
}

#[test]
fn only_a_prepared_release_should_create_a_verified_app_commit_and_tag() {
    let prepare = lines_of(TRAIN, "jobs", "prepare").join("\n");
    let after = prepare.split("name: Create the release bot's token").nth(1).unwrap();
    assert_eq!(after.matches("if: steps.version.outputs.ready == 'true'").count(), 2);
    for part in [
        "actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510",
        "GH_TOKEN: ${{ steps.bot.outputs.token }}",
        "content: $content}",
        "git --literal-pathspecs ls-tree HEAD",
        "git/trees\" --input \"$1\"",
        "git/commits\" --input -",
        ".verification.verified == true and .verification.reason == \"valid\"",
        "ref=\"refs/heads/$BRANCH\"",
        "ref=\"refs/tags/$TAG\"",
    ] {
        assert!(after.contains(part), "missing push safeguard {part}");
    }
    // Inline content, tree requests of 50 entries: a blob request per file passes GitHub's
    // limit on content-creating requests, and one request for the whole commit timed out
    // (docs/bugs/release-tree-request-502). Only binary files take a blob request.
    assert_eq!(after.matches("git/blobs\" --input -").count(), 1);
    assert!(after.contains("Binary content can't go inline"));
    assert!(
        after.contains("\"${files[@]:i:50}\""),
        "tree entries go 50 to a request"
    );
    assert!(
        after.contains("--arg base_tree \"$signed_tree\""),
        "each batch builds on the last"
    );
    // A 4xx answer won't change on a retry: only GitHub-side failures are sent again.
    assert!(after.contains("grep -q 'HTTP 4[0-9][0-9]'"));
    let tree = after.find("git/trees\" --input \"$1\"").unwrap();
    let check = after.find("test \"$signed_tree\" = \"$tree\"").unwrap();
    let sign = after.find("git/commits\" --input -").unwrap();
    let verify = after.find(".verification.verified == true").unwrap();
    let move_branch = after.find("ref=\"refs/heads/$BRANCH\"").unwrap();
    let tag = after.find("ref=\"refs/tags/$TAG\"").unwrap();
    assert!(
        tree < check && check < sign && sign < verify && verify < move_branch && move_branch < tag,
        "the tag must be created last"
    );
    for custom_identity in ["author:", "committer:", "signature:"] {
        assert!(
            !after.get(sign..move_branch).unwrap().contains(custom_identity),
            "GitHub can sign for the App only without {custom_identity}"
        );
    }
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
