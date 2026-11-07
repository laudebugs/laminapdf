# P4 — Docs that match the release: the version on every page, `main` at `next.docs.laminapdf.com`

| Field | Value |
| --- | --- |
| ID | `P4` |
| Status | `done` |
| Plan | [releases §4](../../plans/releases.md#4-docs-that-match-a-version) |
| Round / milestone | Publishing |
| Depends on | [P3](./P3-registry-checks.md) |
| ADRs | Implements [0017 §7](../../decisions/0017-versioning-and-release.md#7-publishing)'s docs item and [0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology)'s docs hosts (amended 2026-11-05) |
| Branch | `docs/p4-docs-match-the-release` |
| PR title | `docs: name the version on every page and serve main's docs at next.docs.laminapdf.com` |
| Size | M (≈ 300–500 lines: the docs site's header and install lines, workflow changes for the owner, tests) |
| PR | [#267](https://github.com/laudebugs/laminapdf/pull/267) |

## Goal

Make every docs deployment match a package version. Every page of the docs site names the version it documents, and its install lines name that exact version. `docs.laminapdf.com` deploys from each release once the packages are on the registries, and `main`'s docs move to `next.docs.laminapdf.com`, marked as unreleased.

## Context

- [Releases plan §4](../../plans/releases.md#4-docs-that-match-a-version), and §5's certificate risk.
- [P3](../done/P3-registry-checks.md) rewrites the install pages, quick starts and Your first PDF to install from the registries; this todo pins their versions.
- `.github/workflows/docs.yml` (previews per pull request, production on pushes to `main`) and `apps/docs/README.md` (the owner's setup).
- The shared site header (`apps/docs/src/components/Header.astro`, `@laminapdf/site-kit`).

## Scope

**In scope**

- The version, read at build time from the fixed group's version (`packages/sdk/package.json`), in the docs site's header; on a `next` build, a notice that the page describes unreleased changes, with a link to the released docs.
- Install lines with the exact version beside the `alpha` tag, from the same value: the install page, the quick starts and Your first PDF.
- `docs.yml`: pushes to `main` deploy the Worker `lamina-pdf-docs-next` (`next.docs.laminapdf.com`, `noindex`); production leaves this workflow. A reusable deploy that the release lane calls after publishing (the release lane's side is P6's, or this todo's if P6 merges first).
- `apps/docs/wrangler.toml` (or a second config) for the `next` Worker and its custom domain; the owner's setup in `apps/docs/README.md`.

**Out of scope**

- Docs for older versions and a version switcher (with `1.0`, [releases plan §4](../../plans/releases.md#4-docs-that-match-a-version)).
- The daily release itself → [P6](../ongoing/P6-daily-release.md).

## Tasks

- [x] Check that a Workers custom domain issues a certificate for `next.docs.laminapdf.com`; if not, ask the owner (advanced certificate, or another name). It does: a Workers custom domain gets its own certificate ([Cloudflare's custom domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/))
- [x] The version in the header and the `next` notice; exact-version install lines
- [x] The `next` Worker's config and the workflow changes, given to the owner as files
- [x] Tests: the built pages carry the version, `next` builds carry the notice and `noindex`, install lines match the version
- [x] README and `CLAUDE.md` (the docs workflow paragraph)

## Acceptance criteria

- [x] A production build's every page names the fixed group's version; a `next` build's every page says it is unreleased and links to the released docs
- [x] No install line on the site names a version other than the fixed group's
- [x] With the owner's workflow change in place, a push to `main` deploys `next.docs.laminapdf.com` and leaves `docs.laminapdf.com` alone

## Post-merge checks

- [ ] `next.docs.laminapdf.com` answers with the `main` build after the next merge
- [ ] The next release deploys `docs.laminapdf.com` with its version

## Tests

The docs site's build checks (`apps/docs/plugins/`): version on every page, the `next` notice and header, install lines.

## Docs to update

- [x] `apps/docs/README.md` (deploying, the owner's setup), `CLAUDE.md`, [releases plan §7](../../plans/releases.md#7-to-dos)

## Risks and open questions

- The second-level name's certificate (above): resolved, a Workers custom domain issues its own.
- Between a merge and the next release, a fix is on `next.docs` only; that is intended.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
