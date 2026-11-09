# K3a — The extension on Firefox

| Field | Value |
| --- | --- |
| ID | `K3a` |
| Status | `ready` |
| Plan | [Browser extension §5 K3](../../plans/browser-extension.md#k3-firefox) |
| Round / milestone | Round K (browser extension) — after M2b |
| Depends on | [K2c](./K2c-chromium-release.md) |
| ADRs | Implements 0033's Firefox sections |
| Branch | `feat/k3a-firefox-extension` |
| PR title | `feat(extension): build, take PDFs over and release on Firefox` |
| Size | M (200–400 changed lines: the Firefox build, its takeover, the source package and the AMO release step) |
| PR | — |

## Goal

Bring the extension to Firefox and through AMO's review. Firefox keeps blocking `webRequest` in Manifest V3, so the extension redirects PDF responses before the built-in pdf.js viewer shows them. Where `filterResponseData` works, the viewer reads the first response and never fetches the file twice. The manifest declares that the extension collects no data. The release ships the source package that AMO's reviewers rebuild, with build steps that give the same bytes on their ARM64 machine.

## Context

- K1a's README: whether the redirect beats pdf.js, whether `filterResponseData` works, and the ARM64 rebuild result.
- ADR 0033: whether Firefox ships PDFium or hayro alone.
- [Distribution note](../../research/browser-extension/notes/distribution-and-tooling.md): AMO's source rules, Ubuntu 24.04 ARM64 with Node 24, the 200 MB source limit, `web-ext sign`.
- [Permissions note](../../research/browser-extension/notes/permissions-privacy-enterprise.md): `data_collection_permissions` (`"required": ["none"]`, Firefox 140+) and revocable host access.

## Scope

**In scope**

- WXT's Firefox target: the event page background, `browser_specific_settings` with the add-on ID, `data_collection_permissions`.
- Takeover by `webRequest.onHeadersReceived` with `blocking`; the first-response path through `filterResponseData` if K1a proved it.
- The source zip and a `SOURCE.md` with a container build that reproduces the package on ARM64.
- The release workflow's AMO step (`wxt submit` or `web-ext sign` with the source zip).
- Playwright or `web-ext run` tests for the Firefox takeover.

**Out of scope** (and which todo covers it)

- Firefox for Android: later ([plan §10](../../plans/browser-extension.md#10-later)).

## Tasks

- [ ] Add the Firefox target and its takeover code.
- [ ] Write the source package step and `SOURCE.md`; rebuild in the ARM64 container and compare hashes in CI.
- [ ] Add the AMO step to the release workflow; the owner creates the AMO listing.
- [ ] Try the takeover by hand in Firefox release and ESR.

## Acceptance criteria

- [ ] In Firefox, a PDF link on a host without CORS opens in Lamina PDF with automatic opening on.
- [ ] The ARM64 container rebuild of the source package gives the same extension zip hash as CI.
- [ ] AMO accepts the first submission.

## Post-merge checks

- [ ] The listing goes live on addons.mozilla.org.

## Tests

The Firefox takeover cases from K2b's suite; the reproducibility check.

## Docs to update

- `apps/extension/README.md`, `RELEASE.md`, and the plan (tick K3a).

## Risks and open questions

- Mozilla changes its add-on policies on 2026-12-01; check them before the first submission.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
