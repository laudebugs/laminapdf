# U5c — Opt-in, content-free usage telemetry

| Field | Value |
| --- | --- |
| ID | `U5c` |
| Status | `ready` |
| Plan | [UI/UX §6 U5. Product promises](../../plans/ui-ux.md#u5-product-promises) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [C7c](./C7c-performance-gates-and-launch.md), [D7a](./D7a-crash-reporting.md) |
| Blocks | None |
| ADRs | Follows ADR 0011 (owner decision 2026-10-29: telemetry is optional, opt-in and off by default; none in v0.1) |
| Branch | `feat/u5c-opt-in-telemetry` |
| PR title | `feat(ui): add opt-in content-free usage counts on web and desktop` |
| Size | M (200–400 lines incl. tests) |
| PR | — |

## Goal

The plan's task-split assumption (viewing ≈ 70% of time) is unverified, and it decides which editor modes ship first. The owner decided telemetry is optional (2026-10-29), so add opt-in, aggregate counts of commands run and modes used — never document content, names, URLs or identifiers — so the roadmap follows real use.

## Context

- **Design (Paper):** [S07 Settings window](../../design/paper/desktop-shell.md#s07-settings-window), [H10 Telemetry opt-in and what we collect](../../design/paper/account-cloud-product.md#h10-telemetry-opt-in-and-what-we-collect). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §3 and §5.9](../../plans/ui-ux.md#3-users); [research: task split](../../research/product/pdf-app-product-and-ux.md#the-70--20--8--2-task-split-is-unverified-and-probably-wrong-for-the-web).
- [web-app plan: privacy and security](../../plans/web-app.md#privacy-and-security) — v0.1 has no telemetry; cookieless aggregate counts and opt-in error reports only, decided in ADR 0011.
- [U1b](./U1b-command-registry-and-palette.md) — command ids are the unit counted.

## Scope

**In scope**

- Both shells: the counter lives in `@laminapdf/ui` on the command registry; each shell sends through its `Platform` adapter (web: our origin; desktop: the same endpoint from the main process, next to D7a's crash-report consent).
- Off by default; a clear opt-in in settings with the exact payload shown (one setting per install, separate from crash reports).
- Payload: daily aggregate counts per command id and mode, app version, platform family; no document data, no user or device identifiers, no IP retention (state and verify the Worker's handling).
- Batching and a single endpoint on our origin (CSP unchanged); the U4a indicator shows these requests.
- A dashboard query documented for the team.

**Out of scope** (and which todo covers it)

- Crash reporting → [D7a](./D7a-crash-reporting.md) (desktop) and ADR 0011 for web.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Check that ADR 0011 records the allowed payload and retention; amend it in a separate PR if it doesn't.
- [ ] Implement the counter on the command registry's run hook; batching and send.
- [ ] Desktop: send path in the main process and the settings toggle next to D7a's consent; WebdriverIO check that nothing is sent when opted out.
- [ ] Tests that the payload contains only allowed keys, and that nothing is sent when opted out.
- [ ] Privacy page update.

## Acceptance criteria

- [ ] With telemetry off, Playwright sees no telemetry request on any flow.
- [ ] With it on, the payload validates against a schema that has no free-text fields (test).
- [ ] The privacy page lists the payload and retention.

## Post-merge checks

None.

## Tests

Unit tests for the payload builder; Playwright opt-out and opt-in flows.

## Docs to update

Privacy page; ADR 0011 (if amended); tick the telemetry item in §9.

## Risks and open questions

- Any telemetry can be read as betraying the privacy promise: keep it off by default, show the exact payload at opt-in, and count its requests in the network indicator (U4a).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
