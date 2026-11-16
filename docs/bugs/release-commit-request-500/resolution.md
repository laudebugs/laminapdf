# Resolution: the daily release fails with a 500 after its tree is built

[Diagnosis](detail.md).

## The fix

Every request in the publish step of `.github/workflows/release-train.yml` goes through one helper, `api`. It sends a request again when GitHub fails it (a 5xx answer or a timeout), 15 and then 30 s later, and fails at once on a 4xx answer, which a retry can't change. Each request's body now comes from a file, so a retry sends it again.

Sending each request again is safe:

- **Blobs and trees** are named by their content, so a second request makes nothing new.
- **The commit** sent again may make a second commit object. No ref points at it, so GitHub drops it, and the step goes on with the commit the last request returned.
- **The read-back** of the commit only reads.
- **A ref** whose request failed on GitHub's side may still have been made. Its retry then answers 422, so `create_ref` reads the ref back and counts it as made when it points at the signed commit. A ref pointing anywhere else still fails the step.

## How it was checked

- **A stand-in `gh`** ran the helpers out of the workflow. A request answered 500 once was sent again and passed. One answered 422 failed after one try. A ref request that answered 500 and then 422 counted as made when the ref pointed at the signed commit, and failed when it pointed elsewhere.
- **Guard test**: xtask's `only_a_prepared_release_should_create_a_verified_app_commit_and_tag` now pins that `gh api` appears once in the step, inside `api`, so every request is retried. It also pins the commit and ref requests in their new forms, still in order. The reviewed copy `tools/release/release-train.yml` matches the installed workflow.
- **Lint**: `actionlint`, with `shellcheck` on the step's script, reports nothing in the step.

## Left for later

- **No test that runs the step.** As before, the guard test reads the workflow's text; the stand-in check is the evidence for the helpers.
- **The failed release.** `v0.5.0` was never published and left no refs. Once this merges, the next Daily release run, or a manual run with the nightly's run ID, publishes it.
