# K2c — Release to the Chrome Web Store and Edge Add-ons

| Field | Value |
| --- | --- |
| ID | `K2c` |
| Status | `ready` |
| Plan | [Browser extension §5 K2](../../plans/browser-extension.md#k2-chromium-chrome-edge-brave-vivaldi-opera) |
| Round / milestone | Round K (browser extension) — **M2b Browser extension** (milestone closes with this PR) |
| Depends on | [K2b](./K2b-automatic-pdf-takeover-chromium.md) |
| ADRs | Implements 0033's release and signing sections. Follows [0017](../../decisions/0017-versioning-and-release.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `ci/k2c-extension-release` |
| PR title | `ci(extension): release the extension to the Chrome Web Store and Edge Add-ons` |
| Size | M (200–400 changed lines: a workflow, its tests, the owner's runbook and the listings' texts) |
| PR | — |

## Goal

Publish the extension where most users are. A release workflow builds the package from a tag and publishes it to the Chrome Web Store and Edge Add-ons with `wxt submit`. It signs Chrome uploads with Lamina's own key, so a stolen store account can't publish an update. Each release publishes the package's SHA-256 hashes so anyone can check that the store's copy matches the source. One Chrome Web Store listing also reaches Brave, Vivaldi and Opera on desktop.

## Context

- [Distribution note](../../research/browser-extension/notes/distribution-and-tooling.md): store fees, review, the Chrome Web Store API v2, the Edge Add-ons API, staged rollout above 10,000 users.
- [Permissions note](../../research/browser-extension/notes/permissions-privacy-enterprise.md): the Limited Use statement, privacy declarations, verified CRX uploads, the Cyberhaven attack.
- [ADR 0017](../../decisions/0017-versioning-and-release.md) and `tools/release/README.md`: how the project's other releases run.
- `apps/web/DEPLOY.md`: the reader's deploy, for the shape of an owner runbook.

## Scope

**In scope**

- `.github/workflows/extension-release.yml` on `extension-v*` tags (or the scheme ADR 0033 chose): build, test, zip, hashes, `wxt submit` to both stores, the GitHub release with the hashes.
- An owner runbook in `apps/extension/RELEASE.md`:
  - the store accounts, their fees and two-factor sign-in;
  - creating the first listing by hand;
  - the signing key and where it is kept;
  - the API credentials as repository secrets;
  - the privacy forms, answered "no data collected".
- Listing text and screenshots from the reader, drafted with the `/writing` skill.
- A page on the website or docs that links to both listings and states the privacy promise.

**Out of scope** (and which todo covers it)

- Firefox and Safari: [K3a](./K3a-firefox-extension.md), [K4a](./K4a-safari-extension.md).
- Enterprise docs and a managed-storage schema: later ([plan §10](../../plans/browser-extension.md#10-later)). This todo only answers [plan §9](../../plans/browser-extension.md#9-open-questions)'s question 8, below.

## Tasks

- [ ] Write the workflow, with xtask tests that it publishes only on a tag push, as `tools/xtask/src/release.rs` does for the library.
- [ ] Write the runbook; the owner creates the accounts and the first listings.
- [ ] Add a reproducibility check: building twice from the tag gives the same zip hash.
- [ ] Force-install the extension on a managed Chrome profile with `ExtensionSettings` and record whether the optional access to all sites is granted (plan §9, question 8) in `RELEASE.md`.
- [ ] Publish the first release once the owner says so.

## Acceptance criteria

- [ ] A dispatched dry run builds, tests and hashes the package without publishing.
- [ ] Two builds from one commit give identical hashes.
- [ ] The listings declare no data collection and match ADR 0033's permission list.

## Post-merge checks

- [ ] The first tag publishes to both stores, and both listings go live.
- [ ] The extension installs from the Chrome Web Store into Brave and Vivaldi.

## Tests

xtask tests of the workflow's publish gate; the reproducibility check in the workflow.

## Docs to update

- CLAUDE.md (the workflow), `apps/extension/README.md`, the website or docs page, and the plan (tick K2c).

## Risks and open questions

- Chrome Web Store review times vary, and Google acknowledged longer delays in April 2026. Plan the first release with slack.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
