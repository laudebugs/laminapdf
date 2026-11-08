# B7c — The docs site at `docs.laminapdf.com`, and the domain filled in everywhere

| Field | Value |
| --- | --- |
| ID | `B7c` |
| Status | `done` |
| Plan | [pdf-library §7 B7](../../plans/pdf-library.md#b7-docs-examples-and-first-release) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B7a](./B7a-docs-site.md) |
| ADRs | Fills in [0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology)'s `<domain>` placeholders, as it asks |
| Branch | `docs/domain-laminapdf` |
| PR title | `docs: laminapdf.com, with the docs site at docs.laminapdf.com` |
| Size | S (one route, one config line, ≈ 40 lines of docs) |
| PR | [#122](https://github.com/laudebugs/lamina/pull/122) |

## Goal

The owner chose the domain `laminapdf.com` (2026-10-31) and put its zone on Cloudflare. Serve the docs site at `docs.laminapdf.com` and replace every `<domain>` placeholder in the plans, ADRs and todos, as ADR 0011 §1 asks.

## Context

- [B7a](./B7a-docs-site.md) — the docs site, its `wrangler.toml` and the owner's setup in `apps/docs/README.md`.
- [ADR 0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology) — the hostnames and "one PR replaces every placeholder".

## Scope

**In scope**

- `apps/docs/wrangler.toml`: a Workers custom domain route for `docs.laminapdf.com`; workers.dev stays on for the PR previews.
- `apps/docs/astro.config.mjs`: `site` (canonical links, sitemap).
- The owner's setup in `apps/docs/README.md` (the token's zone permission), the comment in `.github/workflows/docs.yml` (committed by the owner), CLAUDE.md.
- Every `<domain>` in `docs/` (plans, ADRs 0011 and 0026, design, research, todos); web-app §8 Q1 and architecture §8 Q3 answered; the roadmap's decisions.

**Out of scope** (and which todo covers it)

- The reader at `laminapdf.com` and `staging.laminapdf.com` → [C4d](../ongoing/C4d-deploy-previews-and-production.md).
- The HSTS preload submission: it needs the apex served with the header → C4d.
- Moving `codes.json`'s `docsUrl` base to the docs site's code pages: it changes every surface's URLs, so it waits until production serves them.

## Tasks

- [x] Route `docs.laminapdf.com` to `lamina-docs` and set the site URL.
- [x] Replace every `<domain>` placeholder (`git grep '<domain>'` finds only the history).

## Acceptance criteria

- [x] The site builds with every internal link valid; `cargo xtask check docs` passes.

## Post-merge checks

- [ ] The production deploy from `main` attaches the custom domain and `https://docs.laminapdf.com` serves the site.

## Tests

None beyond the docs site's build (`cargo xtask check ts`).

## Docs to update

`apps/docs/README.md`, CLAUDE.md, the plans, ADRs and todos listed in Scope.

## Risks and open questions

- The deploy fails if the token lacks Zone → Workers Routes → Edit on `laminapdf.com`, or if a DNS record for `docs` already exists; the README says what to do.

## Definition of done

- [x] Claimed: the owner asked for the work directly (2026-10-31)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed
- [ ] CI green
- [x] This file in `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
