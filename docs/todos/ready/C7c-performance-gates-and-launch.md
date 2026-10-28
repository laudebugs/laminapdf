# C7c — Performance budgets as CI gates, privacy page, and the `web v0.1` launch

| Field | Value |
| --- | --- |
| ID | `C7c` |
| Status | `ready` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — **M2 Web reader v0.1** (milestone closes with this PR) |
| Depends on | [C3c](./C3c-viewer-perf-harness.md), [C4d](./C4d-deploy-previews-and-production.md), [C6c](./C6c-print-download-properties.md), [C7a](./C7a-accessibility-and-theming.md), [C7b](./C7b-offline-pwa.md) |
| Blocks | [E1a](./E1a-editing-foundation-feature-plan.md), [E13e](./E13e-server-jobs-infrastructure.md), [H2](./H2-accounts-and-login.md) |
| ADRs | Follows 0011 (publishes its privacy rules as a user-facing page), [0018](../../decisions/0018-test-strategy.md) §2 (which gates run in which lane) |
| Branch | `feat/c7c-web-v0-1-launch` |
| PR title | `feat(web): enforce performance budgets and launch the web reader v0.1` |
| Size | M (200–600 lines: CI gates, privacy page, release notes) |
| PR | — |

## Goal

Ship **web v0.1**. Turn the §4 performance targets into CI gates (app-shell size on every PR; runtime budgets in the full/nightly lanes on the reference configuration), fix or explicitly waive anything still over budget, publish the privacy page that states ADR 0011's guarantees in plain language, tag `web-v0.1.0`, and deploy to production. After this PR, M2 is done and the editor and desktop rounds can rely on the reader packages.

## Context

- [web-app.md §4 non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) and [privacy and security](../../plans/web-app.md#privacy-and-security).
- [C3c](./C3c-viewer-perf-harness.md) — harness and history; open perf issues it filed.
- [ADR 0018 §1 tier 11 and §2](../../decisions/0018-test-strategy.md#2-ci-lanes) — no wall-clock assertions in PR lanes; perf in nightly; regressions open issues. The size budget (bytes) is deterministic and **can** gate PRs; time budgets gate only the nightly/release lanes on a controlled runner.
- ADR 0011 — privacy and telemetry decisions (no telemetry in v0.1 unless the owner decided otherwise).
- [Roadmap milestones](../../plans/roadmap.md#milestones) — M2 Web reader v0.1.

## Scope

**In scope**

- Size gate (PR lane): app shell JS (excluding WASM) min+gz ≤ 200 KB; WASM sizes ≤ their budgets (from C2c/B6); fails the build when exceeded; prints a per-chunk table.
- Runtime gates (release lane, and nightly reporting): first page ≤ 700 ms warm (10-page 1 MB), ≤ 3 s cold Fast 4G; 1,000-page scroll with no main-thread task > 50 ms and sharp within 300 ms; memory ≤ 600 MB and stable; search first result ≤ 500 ms and full pass ≤ 10 s. Release blocks if the reference-machine run fails.
- Lighthouse CI (performance, accessibility, best practices, PWA installability) against the production-like preview, with thresholds.
- Privacy page: `apps/web` route `/privacy` (static, part of the SPA or a static HTML asset) summarising: no upload, documents processed on device, `#url=` fragments never reach our server, no cookies, no third-party scripts, no telemetry (or exactly what, per ADR 0011), what is stored locally and how to clear it, how future opt-in hosted features (accounts, cloud storage) will work and that they are optional; linked from the app footer/menu and the README.
- Release: `apps/web` version `0.1.0`, `CHANGELOG.md` for the app, git tag `web-v0.1.0`, production deploy via C4d's pipeline, post-deploy smoke + privacy suite against production.
- Launch checklist in the PR: all C todos in `done/`, browsers matrix (last two versions of Chrome, Edge, Firefox, Safari desktop and iPadOS) manually smoke-tested, AT pass from C7a recorded, known issues listed.

**Out of scope** (and which todo covers it)

- Publishing `@lamina/*` packages to npm (stays local; [architecture: Publishing (later)](../../plans/architecture.md#publishing-later)).
- Domain purchase/naming if still open ([web-app §8 Q1](../../plans/web-app.md#8-open-questions)) — owner action; launch can use the workers.dev hostname if needed.
- Accounts/login (hosted-service rounds after the `auth` spike).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`.
- [ ] Size-budget script (`tools/` or `cargo xtask size web`) reading Vite's build manifest; min+gz each JS chunk loaded on startup (exclude lazy chunks only if they're truly lazy); CI step in the fast lane for PRs touching `apps/web` or `packages/`.
- [ ] Promote C3c scenarios to gates in the release workflow (on the reference runner or a dedicated self-hosted/large runner); nightly keeps reporting and opening issues.
- [ ] Lighthouse CI config (`lighthouserc.json`) with thresholds (accessibility ≥ 95, best practices ≥ 95, PWA installable, performance ≥ 90 on desktop profile) run against the PR preview (non-required) and release (required).
- [ ] Fix or waive outstanding perf issues from C3c/C5b/C7b (each waiver: issue link, owner, target version) — no silent misses.
- [ ] Write the privacy page content (plain language, reviewed against ADR 0011 point by point) and add it to the app; add a "Privacy" link in the UI and the README.
- [ ] `CHANGELOG.md` for `apps/web`; bump version; release notes (features, browsers, known issues, privacy summary).
- [ ] Tag `web-v0.1.0` after merge; confirm production deploy, smoke, and privacy suite against production.
- [ ] Update plan statuses: [web-app.md](../../plans/web-app.md) status → Done (or In progress with remaining items), [roadmap milestones](../../plans/roadmap.md#milestones) M2 marked shipped, [architecture §7](../../plans/architecture.md#7-to-dos) Round C items ticked.

## Acceptance criteria

- [ ] PR lane fails when the app shell exceeds 200 KB min+gz (demonstrated by a temporary test commit in the PR, then reverted).
- [ ] **Performance targets in §4 are met on the reference machine** (release-lane run attached to the PR) or each miss has an approved waiver issue.
- [ ] Lighthouse thresholds pass on the preview; PWA installability passes.
- [ ] axe and the privacy suite pass against production after deploy.
- [ ] The privacy page is live at `/privacy`, linked from the app, and every statement maps to an ADR 0011 rule and to a test (C4c privacy suite) where testable.
- [ ] Tag `web-v0.1.0` exists; production serves the tagged build (version visible in the About/properties area).
- [ ] All C1a–C7b todos are in `docs/todos/done/`.

## Tests

Size-budget script unit tests; release-lane perf gates (C3c harness); Lighthouse CI; post-deploy smoke + privacy + axe suites against production.

## Docs to update

`apps/web/CHANGELOG.md`, `apps/web/README.md`, root `README.md` (link to the hosted reader and privacy page), `CLAUDE.md` (release/size commands), [web-app.md](../../plans/web-app.md) status and §9 ticks ("Performance budgets as CI gates", "Publish the privacy page; tag `web v0.1` and deploy to production"), [roadmap.md](../../plans/roadmap.md) M2, [architecture.md §7](../../plans/architecture.md#7-to-dos) Round C.

## Risks and open questions

- **Reference machine for runtime gates**: GitHub-hosted runners are noisy; a self-hosted runner or a fixed large runner is needed for gating; until then, gates run manually on the maintainer's reference machine and results are attached (decide with the owner).
- **Telemetry and domain** questions ([§8](../../plans/web-app.md#8-open-questions) Q1, Q3) must be settled before launch copy is final.
- **Safari/iPadOS** issues found late may force waivers; budget a stabilisation week before tagging.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
