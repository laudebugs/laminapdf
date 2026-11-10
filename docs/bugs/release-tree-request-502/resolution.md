# Resolution: the daily release fails with a 502 while publishing its signed commit

[Diagnosis](detail.md).

## The fix

The publish step in `.github/workflows/release-train.yml` now builds the release tree in requests of 50 entries, each on top of the tree the request before returned. Yesterday's 289 files take six requests, well under GitHub's 80-a-minute limit on content-creating requests, and each request stays a fraction of the size that timed out.

Three smaller changes keep the batches safe:

- **Retries.** A tree request that fails is sent again up to three more times, 15, 30 and 45 s apart. Repeating it is safe, since the same entries on the same base give the same tree.
- **Deletions first.** A file that becomes a folder shows up as a deletion plus additions under the same path. Sending deletions first keeps the old file out of the way when the additions land in a later request.
- **Ordered entry files.** Entry files get zero-padded names (`000001.json`), so the shell's glob lists them in the order the step wrote them.

The step still checks that GitHub's final tree equals the commit's tree, byte for byte, before it creates the commit.

## How it was checked

- **Against a real release**: the step, cut off before the commit request, ran on `v0.2.0-alpha.4` (289 files, one binary) with a stand-in `gh` that builds each tree with `git read-tree` and `git update-index`. Its first tree request answered 502. The step warned, retried, sent six batches and ended on `3daafd49`, the commit's own tree.
- **Lint**: `actionlint` passes on the workflow, and `shellcheck` passes on the step's script.

## Left for later

The other requests in the step (the binary blob upload, the commit and the two refs) still run once. They are small, and repeating a commit or ref request isn't idempotent, so they need their own handling if GitHub fails them too.
