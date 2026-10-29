# H2 — Accounts and login for the hosted service

| Field | Value |
| --- | --- |
| ID | `H2` |
| Status | `ready` |
| Plan | [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features) (accounts) |
| Round / milestone | Hosted service foundation (accounts, cloud storage, sync) — before the first round that needs accounts |
| Depends on | [C7c](./C7c-performance-gates-and-launch.md), [D5b](./D5b-updater-in-app.md), [H1](./H1-auth-spike.md) |
| Blocks | [E18b](./E18b-collaboration-feature-plan.md), [H3](./H3-cloud-document-storage.md) |
| Dependency notes | [H1](H1-auth-spike.md), `C7` ([web-app §C7](../../plans/web-app.md#c7-accessibility-offline-and-launch): web reader v0.1), `D5` ([desktop-app §D5](../../plans/desktop-app.md#d5-update-service): update Worker) |
| ADRs | Implements ADR 0011 (as amended by H1) |
| Branch | `feat/h2-accounts-and-login` |
| PR title | `feat(hosted): user accounts and sign-in on web and desktop` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Ship opt-in accounts: sign-up/sign-in/sign-out with the H1 stack on the `apps/web` Worker (`/api/auth/*`, `run_worker_first`), D1 schema for users and sessions, desktop sign-in via the system browser, account settings (delete account, export data), and a privacy page update. The apps keep working fully without an account.

## Context

- [architecture §8 Q2](../../plans/architecture.md#8-open-questions-for-the-team)
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- ADR 0011 (topology, privacy, retention) and the H1 result
- [report: Cloudflare section](../../research/architecture/app-and-package-architecture.md#cloudflare-hosts-everything-for-about-5-a-month-with-heavy-jobs-in-containers) (`run_worker_first: ["/api/*"]`)
- [desktop D5](../../plans/desktop-app.md#d5-update-service): the Worker/R2 setup patterns

## Scope

**In scope**

- `/api/auth/*` routes; D1 migrations (users, sessions, credentials) with Wrangler
- Sign-in UI in `@lamina/ui` (hidden behind "Sign in" — never required)
- Desktop: deep-link callback, token stored in the OS keychain
- Account deletion (hard delete within N days) and data export
- Security headers/CSRF; rate limits; audit log minimal

**Out of scope** (and which todo covers it)

- Cloud storage of documents → [H3](H3-cloud-document-storage.md)
- Sharing and collaboration → [E18f](E18f-sharing-and-notifications.md)

**Editor facts**

- **Crates and packages:** `apps/web` (Worker routes), `@lamina/ui`, `apps/desktop/src-tauri`
- **`SaveConstraint`:** not applicable (no PDF writes)
- **Web vs desktop:** web: cookie session; desktop: system-browser sign-in + keychain token
- **Server features:** Workers, D1 (and the vendor if H1 chose one)
- **Capability-matrix rows to flip:** none (not a PDF operation)

## Tasks

- [ ] D1 schema and migrations
- [ ] Routes
- [ ] Web UI
- [ ] Desktop flow
- [ ] Deletion/export
- [ ] Privacy page

## Acceptance criteria

- [ ] e2e: sign up, sign in, sign out, delete account on web; desktop sign-in on all three OSes
- [ ] Without signing in, every feature that worked before still works and no request goes to `/api/*` (Playwright)
- [ ] Session cookies `Secure; HttpOnly; SameSite`; CSRF tests

## Post-merge checks

- [ ] D1 migrations (users, sessions, credentials) applied to production and the `apps/web` Worker deployed with `/api/auth/*` by the deploy workflow on `main` (link the run); sign-up, sign-in and sign-out work on production

## Tests

- Worker tests (Miniflare + D1 local)
- Playwright and WebdriverIO e2e

## Docs to update

- Privacy page
- ADR 0011 status
- Ops runbook (secrets, migrations)
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features) (accounts): tick what this PR completes and point the todo link at `done/`

## Risks and open questions

- Open product questions: pricing/plans (lamina will be priced, owner decision 2026-10-29; the model, tiers and whether accounts check licences are decided once, in [U5a](./U5a-pricing-and-promises-page.md)), age requirements, terms of service, data residency — decide before launch
- Desktop deep links differ per OS; test each

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
