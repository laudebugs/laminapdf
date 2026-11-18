# Resolution: the release trains fail with a 403 creating their refs

[Diagnosis](detail.md).

## The fix

The release bot's token in `release-train.yml`, `web-release-train.yml` and `cli-release-train.yml` asks for `permission-workflows: write` beside `permission-contents: write`, in `.github/workflows/` and in the reviewed copies in `tools/release/`. The CLI train tags the release commit too, so it would meet the same refusal.

The App must have the Workflows permission (read and write) on its installation before this merges: a token that asks for a permission the installation lacks is refused, and the trains would then fail at the token step. `tools/release/README.md` lists it.

## How it was checked

- **Guard test**: xtask's `every_train_bot_token_should_be_allowed_to_create_refs_past_workflow_changes` reads each train's token step and fails without both permissions. The installed workflows still match the reviewed copies.
- **Not checked**: no train has run with the new permission. The next Daily release and Web reader release runs, or runs dispatched with the 2026-11-18 nightly's ID, are the evidence; if they still answer 403, look for a ruleset (see the diagnosis).

## Left for later

- **The failed releases.** `v0.6.0` and `web-v0.1.9` were never made. Once the permission is granted and this merges, a run of each train with the nightly's run ID makes them.
