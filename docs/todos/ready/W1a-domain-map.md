# W1a — The domain map: the website at `laminapdf.com`, the reader at `app.laminapdf.com`

| Field | Value |
| --- | --- |
| ID | `W1a` |
| Status | `ready` |
| Plan | [Website §2 The domain map](../../plans/website.md#2-the-domain-map) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | None |
| Blocks | [C4c](./C4c-remote-files-encryption-privacy.md), [C4d](./C4d-deploy-previews-and-production.md), [W1b](./W1b-site-scaffold-and-deploys.md) |
| ADRs | Amends [0011 §1, §2 and §4](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology) |
| Branch | `docs/w1a-domain-map` |
| PR title | `docs: the website at laminapdf.com and the reader at app.laminapdf.com` |
| Size | S (100–250 lines of docs) |
| PR | — |

## Goal

Every plan, ADR, todo and design doc says the same thing about who serves which host: the website at `laminapdf.com`, the reader at `app.laminapdf.com` with its staging at `staging-app.laminapdf.com`, and the docs, updater and downloads where they are. The reader hasn't been deployed, so this PR changes only documents, and C4b, C4c and C4d start from the new names.

## Context

- The owner's request (2026-11-05): "the app rests on app.laminapdf.com and laminapdf.com is the landing page".
- [Website plan §2–§4](../../plans/website.md#2-the-domain-map): the domain map, why the reader gets its own origin, and the website's pages and privacy rules.
- [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md): §1 puts the reader at the apex and holds the HSTS preload for it; §2 gives `https://laminapdf.com/#url=…`; §3's guarantees speak of "our origin".

## Scope

**In scope**

- ADR 0011, as an amendment note dated when this PR merges, then the edits in place:
  - §1: the hosts of the [plan's table](../../plans/website.md#2-the-domain-map); the reader's Workers keep their names; the website is a third Worker, `lamina-pdf-site`, deployed from `main` with a preview per PR, which never serves the reader.
  - §2: remote documents open from `https://app.laminapdf.com/#url=…`; the website forwards no fragment.
  - §3: "our origin" is `app.laminapdf.com`; the website's own rules (no cookies, no third parties, no analytics, no Worker code), stated as checks W1b runs on every built page.
  - §4: the website's headers (the reader's, without `'wasm-unsafe-eval'` and with `connect-src 'self'`) and the HSTS preload after the website's first production deploy.
- The plans: [web reader](../../plans/web-app.md) (§3's link row, §5 hosting, §8 Q1), [roadmap](../../plans/roadmap.md) "Decisions made" (the domain line), [desktop plan](../../plans/desktop-app.md) (the download page's home).
- The todos that name hosts or pages: [C4c](./C4c-remote-files-encryption-privacy.md) (`#url=` links and the privacy suite's origin), [C4d](./C4d-deploy-previews-and-production.md) (the custom domains), [C7c](./C7c-performance-gates-and-launch.md) (the privacy page moves to the website; the reader links to it; C7c turns on "Open the web app"), [D7b](./D7b-settings-renderer-switch-and-download-page.md) (`/download` in `apps/site`), [U5a](./U5a-pricing-and-promises-page.md) and [U5b](./U5b-reader-benchmark.md) (website pages, not docs-site pages), [D5a](./D5a-update-worker-and-adr-0014.md) (release notes under the website).
- The design docs: [design language §7.39](../../design/paper/design-language.md#739-site-and-product-pages-proposed) ("site header" for the website and the docs site), [H08](../../design/paper/account-cloud-product.md#h08-pricing-and-promises-page) (a website page), [desktop shell](../../design/paper/desktop-shell.md) (entry points).
- `CLAUDE.md`'s ADR 0011 summary.

**Out of scope**

- Building `apps/site` → [W1b](./W1b-site-scaffold-and-deploys.md).
- Done todos, which keep the names they were written with.

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] Amend ADR 0011 as in Scope.
- [ ] Search `docs/`, `CLAUDE.md` and `apps/` for `laminapdf.com` and `staging.laminapdf.com`; change every live reference, leaving `docs.`, `updates.` and `downloads.` as they are.
- [ ] Regenerate the board.
- [ ] `cargo xtask check docs`.

## Acceptance criteria

- [ ] A search of `docs/` (outside `done/`) and `CLAUDE.md` finds `staging.laminapdf.com` nowhere, and finds `laminapdf.com` alone only for the website.
- [ ] ADR 0011 names every host in the plan's table, with the Worker and the deploy trigger for each.
- [ ] Each moved page (`/download`, `/privacy`, `/pricing`, `/performance`, release notes) names `apps/site` as its home in its todo.

## Tests

`cargo xtask check docs` (links, anchors, the board).

## Docs to update

All of Scope; [website plan §8](../../plans/website.md#8-to-dos): tick the first item.

## Risks and open questions

- **Reviewers read ADR 0011 for the privacy promises.** Keep the ten guarantees' wording; change only the origin they name, so C4c's tests map one to one.

## Definition of done

- [ ] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` and `/writing-review` followed (findings and fixes recorded in commit messages)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
