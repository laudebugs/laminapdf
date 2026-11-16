# Resolution: the daily release fails with a 500 after its tree is built

[Diagnosis](detail.md).

## The fix

Every request in the publish step of `.github/workflows/release-train.yml` goes through one helper, `api`. It sends a request again, 15 and then 30 s later, when GitHub fails it (a 5xx answer) or it hangs: each try runs under `timeout 300`, since `gh api` has no timeout of its own. A rate limit (429, or a 403 that says so) is sent again a minute later. Any other 4xx fails the step at once, since a retry can't change it. Three tries of five minutes at most stay inside the job's 30. Each request's body now comes from a file, so a retry sends it again. `post_object` makes a blob, tree or commit through `api` and checks the SHA it returns.

A retry is the fix because the cause is outside the repository: a GitHub server error during a request it normally answers. The step can only make one passing failure not stop the release. Bug triage §6 warns against a retry that makes a failing case pass; here the request itself was never wrong.

Sending each request again is safe:

- **Blobs and trees** are named by their content, so a second request makes nothing new.
- **The commit** sent again may make a second commit object. No ref points at it, so GitHub drops it. The step goes on with the commit the last request returned, and checks its tree, parent and signature before any ref.
- **The read-backs** only read.
- **A ref** whose request failed on GitHub's side may still have been made. Its retry then answers 422, so `create_ref` reads the ref back and counts it as made when it points at the signed commit. A ref pointing anywhere else still fails the step.

## How it was checked

- **A stand-in `gh`** ran the helpers out of the workflow, under bash with the step's `set -euo pipefail`:
  - a read (`GET`) answered 502 once was sent again and passed;
  - a request that hung was cut off by `timeout` and sent again;
  - a secondary rate limit (403) was sent again after 60 s;
  - a 404 failed after one try;
  - a ref request answered 500 and then 422 counted as made when the ref pointed at the signed commit, and failed when it pointed elsewhere.

  The first version of the helper printed `$1 $2 $3` in its warning, which under `set -u` failed every one-argument read on its first error. The review found it, and the read case above covers it.
- **Guard test**: xtask's `only_a_prepared_release_should_create_a_verified_app_commit_and_tag` now pins that `gh api` appears once in the step, as `timeout 300 gh api "$@"` inside `api`, so every request is retried. It also pins the tree, commit and ref requests in their new forms, still in order. The reviewed copy `tools/release/release-train.yml` matches the installed workflow.
- **Lint**: `actionlint`, with `shellcheck` on the step's script, reports nothing in the step.

## Left for later

- **No test that runs the step.** As before, the guard test reads the workflow's text; the stand-in check is the evidence for the helpers.
- **The failed release.** `v0.5.0` was never published and left no refs. Once this merges, the next Daily release run, or a manual run with the nightly's run ID, publishes it.
