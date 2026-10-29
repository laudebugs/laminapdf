# H1 — `auth` spike: Better Auth / Auth.js on D1 vs hosted vendors

| Field | Value |
| --- | --- |
| ID | `H1` |
| Status | `ready` |
| Plan | [architecture §4 Spikes](../../plans/architecture.md#4-spikes-to-close-open-questions) (`auth`) |
| Round / milestone | Hosted service foundation (accounts, cloud storage, sync) — before the first round that needs accounts |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [C4d](./C4d-deploy-previews-and-production.md), [D2e](./D2e-desktop-e2e-webdriverio.md) |
| Blocks | [E18a](./E18a-adr-0012-collaboration.md), [H2](./H2-accounts-and-login.md) |
| Dependency notes | `C4` ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare): `@lamina/ui`, `apps/web`), `C2` ([web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs): `lamina-render`, ADR 0011), `D2` ([desktop-app §D2](../../plans/desktop-app.md#d2-shell-and-native-core): Tauri shell and engine process) |
| ADRs | Feeds ADR 0011 (amends it with the login stack) |
| Branch | `spike/h1-auth-spike` |
| PR title | `spike(auth): compare login stacks for the hosted service` |
| Size | M (200–600) |
| PR | — |

## Goal

Answer the architecture's open spike: which login stack for lamina's hosted service (decided 2026-10-28) — self-hosted Better Auth or Auth.js on Workers + D1, or a hosted vendor (e.g. Clerk, WorkOS) — measured on a throwaway prototype, and record the result in ADR 0011. Must merge before the first round that needs accounts.

## Context

- [architecture §4 Spikes](../../plans/architecture.md#4-spikes-to-close-open-questions)
- [architecture §8 Q2](../../plans/architecture.md#8-open-questions-for-the-team)
- [cloudflare notes §4](../../research/architecture/notes/cloudflare.md#4-privacy-first-architecture-auth-desktop-distribution-cost-observability): auth inferences (unverified: the spike verifies them)
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- [roadmap: decisions made](../../plans/roadmap.md#decisions-made): hosted service; local processing stays default

## Scope

**In scope**

- `spikes/auth/` prototypes: (a) Better Auth on a Worker with D1; (b) Auth.js on a Worker with D1; (c) one hosted vendor
- Criteria: Workers/D1 compatibility, passkeys/WebAuthn, email magic link, OAuth providers, session model (cookies vs tokens), desktop sign-in (system browser + deep link/loopback with PKCE in Tauri), account deletion/export, licence, cost per MAU, data residency, lock-in, maintenance
- README with measurements and a recommendation
- ADR 0011 amendment PR text drafted

**Out of scope** (and which todo covers it)

- Production accounts → [H2](H2-accounts-and-login.md)

**Editor facts**

- **Crates and packages:** spike only (outside workspaces)
- **`SaveConstraint`:** not applicable (no PDF writes)
- **Web vs desktop:** desktop sign-in flow prototyped with Tauri deep links (load `/tauri-v2`)
- **Server features:** Workers + D1 prototypes on a preview account; no production data
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Build the three prototypes
- [ ] Desktop flow prototype
- [ ] Score against criteria
- [ ] Write README and recommendation
- [ ] Owner decision recorded in ADR 0011

## Acceptance criteria

- [ ] README compares all options with evidence
- [ ] ADR 0011 records the chosen stack (owner sign-off)

## Post-merge checks

None.

## Tests

Spike: manual and scripted checks recorded in the README.

## Docs to update

- `docs/plans/architecture.md` §4 (spike done, result link)
- ADR 0011
- [architecture §4 Spikes](../../plans/architecture.md#4-spikes-to-close-open-questions) (`auth`): tick what this PR completes and point the todo link at `done/`

## Risks and open questions

- Open product questions for the owner: which sign-in methods (passkeys only? email? Google/GitHub/Apple?), whether accounts are free (lamina will be priced, owner decision 2026-10-29; the model is set later in [U5a](./U5a-pricing-and-promises-page.md), so pick a login stack that works either way and can carry an entitlements claim), and data residency (EU?)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
