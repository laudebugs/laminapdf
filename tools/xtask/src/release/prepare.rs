//! Prepare a version commit from a full nightly's exact commit. Nothing here pushes a ref.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::util::{gh, git_stdout, github_output};

/// Only `prepare --run <id>`: the workflow names the completed nightly explicitly.
pub(crate) fn run(root: &Path, args: &[String]) -> Result<()> {
    let [command, flag, id] = args else {
        bail!("usage: cargo xtask release prepare --run <nightly-run-id>");
    };
    if command != "prepare" || flag != "--run" || id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        bail!("usage: cargo xtask release prepare --run <nightly-run-id>");
    }
    github_output("ready", "false")?;
    let head = git_stdout(root, &["rev-parse", "HEAD"]).context("reading HEAD")?;
    let nightly = gh(root, &["run", "view", id, "--json", super::FIELDS])?;
    // An ancestor is enough to guard a manual release; preparing one requires the exact tree tested.
    super::verdict(&nightly, &head, &|tested| Ok(tested == head))?;
    if !git(root, &["status", "--porcelain"])?.is_empty() {
        bail!("release prepare requires a clean worktree, including untracked files");
    }
    let group = crate::version::fixed_group_version(root)?;
    ensure_changesets_base(root, &head)?;
    let status = root.join("target/xtask/release/status.json");
    fs::create_dir_all(status.parent().context("release status has no parent")?)?;
    // Remove an earlier report: a failed status command must never leave a usable stale plan.
    if status.exists() {
        fs::remove_file(&status)?;
    }
    crate::run(
        root,
        "pnpm",
        &["changeset", "status", "--output", &status.to_string_lossy()],
    )?;
    let report: Value = serde_json::from_str(&fs::read_to_string(status)?)?;
    let members: Vec<&str> = group.members.iter().map(|(name, _)| name.as_str()).collect();
    let Some(version) = next_version(&report, &members, &group.version)? else {
        println!("No pending changeset: nothing to release.");
        return Ok(());
    };
    let prs = gh(
        root,
        &[
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "1000",
            "--json",
            "headRefName",
        ],
    )?;
    refuse_open_pr(&prs)?;
    let tag = format!("v{version}");
    refuse_remote_refs(root, &tag)?;
    let branch = format!("release/{tag}");
    git(root, &["switch", "-c", &branch])?;
    crate::run(root, "pnpm", &["changeset", "version"])?;
    crate::version::run(root, &crate::version::Command::Sync { check: false })?;
    if crate::version::fixed_group_version(root)?.version != version {
        bail!("Changesets produced a different version from its plan; nothing was pushed");
    }
    crate::publish::crate_docs(root)?;
    git(root, &["add", "--all"])?;
    git(root, &["add", "--force", "crates/laminapdf/docs"])?;
    let tree = git(root, &["write-tree"])?;
    let changes = super::changes_since(root, &head, &tree)?;
    if changes.is_empty() || !super::only_release_changes(&changes, &group.version, &version) {
        bail!("the generated tree changes more than the version and packaged docs; nothing was pushed");
    }
    git(
        root,
        &["-c", "commit.gpgsign=false", "commit", "-m", &format!("release: {tag}")],
    )?;
    github_output("version", &version)?;
    github_output("branch", &branch)?;
    github_output("tag", &tag)?;
    github_output("ready", "true")?;
    println!("Prepared {branch} from nightly {id}; no refs pushed.");
    Ok(())
}

/// Changesets' status command compares packages against its configured base even when it reads
/// all pending changesets. A SHA checkout fetches origin/main but creates no local main branch.
/// Supply a missing base at the tested commit, without moving an existing ref or changing HEAD.
fn ensure_changesets_base(root: &Path, tested: &str) -> Result<()> {
    let path = root.join(".changeset/config.json");
    let config: Value = serde_json::from_str(&fs::read_to_string(&path).context("reading Changesets config")?)?;
    let base = config
        .get("baseBranch")
        .and_then(Value::as_str)
        .context("Changesets config has no baseBranch")?;
    if git_stdout(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("{base}^{{commit}}"),
        ],
    )
    .is_none()
    {
        git(root, &["branch", "--", base, tested])
            .with_context(|| format!("creating missing Changesets base {base:?} at the tested commit"))?;
    }
    Ok(())
}

/// The fixed group's next version from Changesets' release plan, including its pre-mode rules.
fn next_version(report: &Value, members: &[&str], current: &str) -> Result<Option<String>> {
    let changesets = report
        .get("changesets")
        .and_then(Value::as_array)
        .context("status has no changesets")?;
    let releases = report
        .get("releases")
        .and_then(Value::as_array)
        .context("status has no releases")?;
    if changesets.is_empty() {
        return Ok(None);
    }
    let mut next: Option<&str> = None;
    for member in members {
        let release = releases
            .iter()
            .find(|release| release.get("name").and_then(Value::as_str) == Some(member))
            .with_context(|| format!("Changesets' plan omits {member}"))?;
        let version = release
            .get("newVersion")
            .and_then(Value::as_str)
            .context("release has no newVersion")?;
        if !crate::version::is_version(version) || crate::version::compare_versions(version, current).is_le() {
            bail!("Changesets did not advance {member} past {current}: {version}");
        }
        if next.is_some_and(|expected| expected != version) {
            bail!("Changesets' fixed group disagrees about the next version");
        }
        next = Some(version);
    }
    next.map(str::to_owned).map(Some).context("empty fixed group")
}

/// The branch exists before the PR does; a failed release must be recovered, not cut twice.
fn refuse_remote_refs(root: &Path, tag: &str) -> Result<()> {
    if !git(
        root,
        &[
            "-c",
            "credential.helper=",
            "-c",
            "credential.helper=!gh auth git-credential",
            "ls-remote",
            "--heads",
            "origin",
            "refs/heads/release/v*",
        ],
    )?
    .is_empty()
    {
        bail!("a release/v* branch still exists; finish or recover that release first");
    }
    if !git(
        root,
        &[
            "-c",
            "credential.helper=",
            "-c",
            "credential.helper=!gh auth git-credential",
            "ls-remote",
            "--tags",
            "origin",
            &format!("refs/tags/{tag}"),
        ],
    )?
    .is_empty()
    {
        bail!("tag {tag} already exists; refusing to cut it twice");
    }
    Ok(())
}

fn refuse_open_pr(json: &str) -> Result<()> {
    let prs: Value = serde_json::from_str(json)?;
    let prs = prs.as_array().context("gh pr list did not return an array")?;
    for pr in prs {
        let head = pr
            .get("headRefName")
            .and_then(Value::as_str)
            .context("PR has no headRefName")?;
        if head.starts_with("release/v") {
            bail!("version pull request {head} is still open; finish it first");
        }
    }
    Ok(())
}

/// Trim a successful git response; remote failures must never mean an absent tag or branch.
fn git(root: &Path, args: &[&str]) -> Result<String> {
    crate::tracked::git(root, args).map(|text| text.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{next_version, refuse_open_pr};
    use serde_json::{Value, json};

    fn plan(version: &str) -> Value {
        json!({"changesets": [{"id": "fix"}], "releases": [
            {"name": "sdk", "newVersion": version}, {"name": "wasm", "newVersion": version}
        ]})
    }

    #[test]
    fn pending_changeset_should_use_changesets_alpha_version() {
        assert_eq!(
            next_version(&plan("0.2.0-alpha.3"), &["sdk", "wasm"], "0.2.0-alpha.2").unwrap(),
            Some("0.2.0-alpha.3".into())
        );
    }

    #[test]
    fn no_pending_changeset_should_release_nothing() {
        assert_eq!(
            next_version(&json!({"changesets": [], "releases": []}), &["sdk"], "0.2.0-alpha.2").unwrap(),
            None
        );
    }

    #[test]
    fn malformed_or_inconsistent_plan_should_fail() {
        let mut split = plan("0.2.0-alpha.3");
        split["releases"][1]["newVersion"] = json!("0.3.0-alpha.0");
        for report in [
            json!({}),
            split,
            plan("0.2.0-alpha.2"),
            plan("0.2.0-alpha.1"),
            plan("bad"),
        ] {
            assert!(next_version(&report, &["sdk", "wasm"], "0.2.0-alpha.2").is_err());
        }
        assert!(next_version(&plan("0.2.0-alpha.3"), &["missing"], "0.2.0-alpha.2").is_err());
    }

    #[test]
    fn open_version_pr_should_block_even_with_other_prs() {
        assert!(refuse_open_pr(r#"[{"headRefName":"feat/reader"},{"headRefName":"release/v0.2.0-alpha.3"}]"#).is_err());
        assert!(refuse_open_pr(r#"[{"headRefName":"feat/reader"}]"#).is_ok());
        assert!(refuse_open_pr("{}").is_err());
    }
    #[test]
    fn remote_branch_or_tag_should_block_and_connection_errors_should_fail_closed() {
        use crate::tracked::{TempTree, git};
        let tree = TempTree::new("prepare-refs", &[("file", "tested")]);
        let root = tree.0.as_path();
        git(root, &["init", "-q"]).unwrap();
        git(root, &["add", "file"]).unwrap();
        git(
            root,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "tested",
            ],
        )
        .unwrap();
        // No origin is an error, never permission to release.
        assert!(super::refuse_remote_refs(root, "v0.2.0-alpha.3").is_err());
        git(root, &["init", "--bare", "-q", "remote.git"]).unwrap();
        git(root, &["remote", "add", "origin", "remote.git"]).unwrap();
        assert!(super::refuse_remote_refs(root, "v0.2.0-alpha.3").is_ok());
        git(
            root,
            &["push", "-q", "origin", "HEAD:refs/heads/release/v0.2.0-alpha.3"],
        )
        .unwrap();
        assert!(
            super::refuse_remote_refs(root, "v0.2.0-alpha.4")
                .unwrap_err()
                .to_string()
                .contains("branch")
        );
        git(root, &["push", "-q", "origin", ":refs/heads/release/v0.2.0-alpha.3"]).unwrap();
        git(root, &["push", "-q", "origin", "HEAD:refs/tags/v0.2.0-alpha.3"]).unwrap();
        assert!(
            super::refuse_remote_refs(root, "v0.2.0-alpha.3")
                .unwrap_err()
                .to_string()
                .contains("tag")
        );
        assert!(super::refuse_remote_refs(root, "v0.2.0-alpha.4").is_ok());
    }

    #[test]
    fn prepare_should_require_the_exact_tested_commit_and_a_full_successful_nightly() {
        use serde_json::json;
        let run = json!({"databaseId": 42, "conclusion": "success", "status": "completed",
            "event": "schedule", "displayTitle": "Nightly", "headSha": "tested",
            "headBranch": "main", "workflowName": "Nightly"});
        for (field, value) in [
            ("headSha", "other"),
            ("conclusion", "failure"),
            ("conclusion", "cancelled"),
            ("event", "pull_request"),
            ("headBranch", "branch"),
        ] {
            let mut bad = run.clone();
            bad[field] = json!(value);
            assert!(super::super::verdict(&bad.to_string(), "tested", &|sha| Ok(sha == "tested")).is_err());
        }
        assert!(super::super::verdict(&run.to_string(), "tested", &|sha| Ok(sha == "tested")).is_ok());
    }
    /// A detached Actions-style checkout with a remote-tracking main but no local branches.
    fn detached_checkout(name: &str, base: &str) -> crate::tracked::TempTree {
        use crate::tracked::{TempTree, git};
        let config = json!({"baseBranch": base}).to_string();
        let tree = TempTree::new(name, &[(".changeset/config.json", &config)]);
        let root = tree.0.as_path();
        git(root, &["init", "-q", "--initial-branch=main"]).unwrap();
        git(root, &["add", ".changeset/config.json"]).unwrap();
        git(
            root,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "tested",
            ],
        )
        .unwrap();
        let head = git(root, &["rev-parse", "HEAD"]).unwrap();
        git(root, &["update-ref", "refs/remotes/origin/main", head.trim()]).unwrap();
        git(root, &["checkout", "--detach", "-q"]).unwrap();
        git(root, &["branch", "-D", "main"]).unwrap();
        tree
    }

    #[test]
    fn missing_changesets_base_should_be_created_without_leaving_the_tested_commit() {
        use crate::tracked::git;
        let tree = detached_checkout("prepare-missing-base", "main");
        let root = tree.0.as_path();
        let tested = git(root, &["rev-parse", "HEAD"]).unwrap();
        assert!(git(root, &["merge-base", "main", "HEAD"]).is_err());
        super::ensure_changesets_base(root, tested.trim()).unwrap();
        assert_eq!(git(root, &["merge-base", "main", "HEAD"]).unwrap(), tested);
        assert!(git(root, &["symbolic-ref", "-q", "HEAD"]).is_err());
        assert_eq!(git(root, &["status", "--porcelain"]).unwrap(), "");
    }

    #[test]
    fn existing_changesets_base_should_not_be_rewound_to_an_older_nightly() {
        use crate::tracked::git;
        let tree = detached_checkout("prepare-existing-base", "main");
        let root = tree.0.as_path();
        let tested = git(root, &["rev-parse", "HEAD"]).unwrap();
        git(
            root,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-qm",
                "later",
            ],
        )
        .unwrap();
        let later = git(root, &["rev-parse", "HEAD"]).unwrap();
        git(root, &["branch", "main"]).unwrap();
        git(root, &["checkout", "--detach", tested.trim()]).unwrap();
        super::ensure_changesets_base(root, tested.trim()).unwrap();
        assert_eq!(git(root, &["rev-parse", "main"]).unwrap(), later);
        assert_eq!(git(root, &["rev-parse", "HEAD"]).unwrap(), tested);
    }

    #[test]
    fn changesets_base_should_follow_the_configured_branch_name() {
        use crate::tracked::git;
        let tree = detached_checkout("prepare-configured-base", "trunk");
        let root = tree.0.as_path();
        let tested = git(root, &["rev-parse", "HEAD"]).unwrap();
        super::ensure_changesets_base(root, tested.trim()).unwrap();
        assert_eq!(git(root, &["rev-parse", "trunk"]).unwrap(), tested);
        assert!(git(root, &["rev-parse", "--verify", "refs/heads/main"]).is_err());
    }
}
