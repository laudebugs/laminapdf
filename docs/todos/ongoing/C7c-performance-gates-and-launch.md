# C7c — Performance budgets as CI gates, privacy page, and the `web v0.1` launch

| Field | Value |
| --- | --- |
| ID | `C7c` |
| Status | `ongoing` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — **M2 Web reader v0.1** (milestone closes with this PR) |
| Depends on | [C3c](../done/C3c-viewer-perf-harness.md), [C4d](../done/C4d-deploy-previews-and-production.md), [C6c](../done/C6c-print-download-properties.md), [C7a](../done/C7a-accessibility-and-theming.md), [C7b](../done/C7b-offline-pwa.md), [C7d](../done/C7d-cold-first-page.md), [W1b](../done/W1b-site-scaffold-and-deploys.md), [U2a](../done/U2a-selection-surfaces-and-copy-modes.md), [U2c](../done/U2c-page-appearance-modes.md), [U3a](../done/U3a-responsive-mobile-and-touch.md), [U4a](../done/U4a-network-indicator-and-privacy-proof.md) |
| ADRs | Follows 0011 (publishes its privacy rules as a user-facing page), [0018](../../decisions/0018-test-strategy.md) §2 (which gates run in which lane) |
| Branch | `feat/c7c-web-v0-1-launch` |
| PR title | `feat(web): enforce performance budgets and launch the web reader v0.1` |
| Size | M (200–600 lines: CI gates, privacy page, release notes) |
| PR | — |

## Goal

Ship **web v0.1**. Turn the §4 performance targets into CI gates (app-shell size on every PR; runtime budgets in the full/nightly lanes on the reference configuration), fix or explicitly waive anything still over budget, publish the privacy page that states ADR 0011's guarantees in plain language, tag `web-v0.1.0`, and deploy to production. After this PR, M2 is done and the editor and desktop rounds can rely on the reader packages.

## Context

- [UI/UX plan §6](../../plans/ui-ux.md#6-rounds) — the launch also waits for the v0.1 parts of Round U: selection surfaces ([U2a](../done/U2a-selection-surfaces-and-copy-modes.md)), page modes ([U2c](../done/U2c-page-appearance-modes.md)), the phone layout ([U3a](../done/U3a-responsive-mobile-and-touch.md)) and the network indicator ([U4a](../done/U4a-network-indicator-and-privacy-proof.md)). The plan's numbers stay the gates; the research's stretch goals are published separately ([U5a](../ready/U5a-pricing-and-promises-page.md)).
- [web-app.md §4 non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) and [privacy and security](../../plans/web-app.md#privacy-and-security).
- [C3c](../done/C3c-viewer-perf-harness.md) — harness and history; open perf issues it filed. Its first full run (Chromium on a CI container, 2026-11-05; the viewer README's "First measurements") came in under the scrolling and memory targets. It missed the cold first page (4.2 s on Fast 4G, about 3.7 s of it the download: PDFium's WebAssembly 2.2 MB gzipped, the base module 0.4 MB, the 1 MB file) and found zooming in slow (about 1.2 s to sharp, with a 317 ms frame).
- [ADR 0018 §1 tier 11 and §2](../../decisions/0018-test-strategy.md#2-ci-lanes) — no wall-clock assertions in PR lanes; perf in nightly; regressions open issues. The size budget (bytes) is deterministic and **can** gate PRs; time budgets gate only the nightly/release lanes on a controlled runner.
- ADR 0011 — privacy and telemetry decisions (no telemetry in v0.1; optional opt-in telemetry later, owner decision 2026-10-29).
- [Roadmap milestones](../../plans/roadmap.md#milestones) — M2 Web reader v0.1.

## Scope

**In scope**

- Size gate (PR lane): app shell JS (excluding WASM) min+gz ≤ 200 KB; WASM sizes ≤ their budgets (from C2c/B6); fails the build when exceeded; prints a per-chunk table.
- Runtime gates (release lane, and nightly reporting): first page ≤ 700 ms warm (10-page 1 MB), ≤ 3 s cold Fast 4G; 1,000-page scroll with no main-thread task > 50 ms and sharp within 300 ms; memory ≤ 600 MB and stable; search first result ≤ 500 ms and full pass ≤ 10 s. Release blocks if the reference-machine run fails: C4d deploys production **only** from the `web-v*` tag, and its production job `needs:` these release-lane gate jobs, so a failing gate means production is not deployed (staging on `main` already serves the build for testing).
- Lighthouse CI (performance, accessibility, best practices — Lighthouse 12+ has no PWA category) against the production-like preview, with thresholds; installability is gated by C7b's `installability.spec.ts` (manifest schema + fields, icons, SW control, Chromium `Page.getInstallabilityErrors` empty, offline reload) run against the same preview.
- Privacy page: `/privacy` on the website (`apps/site`, a static Astro page; [website plan §4](../../plans/website.md#4-the-website)) summarising: no upload, documents processed on device, `#url=` fragments never reach our server, no cookies, no third-party scripts, no telemetry (or exactly what, per ADR 0011), what is stored locally and how to clear it, how future opt-in hosted features (accounts, cloud storage) will work and that they are optional; linked from the reader's footer/menu (to `https://laminapdf.com/privacy`), the website's header and the README. The privacy promises also join the website's home page, each linking to `/privacy`.
- The website: turn on its "Open the web app" build setting, so the website's header and home page link to `https://app.laminapdf.com` ([website plan §4](../../plans/website.md#4-the-website)).
- Release: `apps/web` version `0.1.0` and `CHANGELOG.md` for the app in this PR; after merge, git tag `web-v0.1.0`, production deploy via C4d's pipeline, post-deploy smoke + privacy suite against production (see Post-merge checks).
- Launch checklist in the PR: all C todos in `done/`, browsers matrix (last two versions of Chrome, Edge, Firefox, Safari desktop and iPadOS) manually smoke-tested, AT pass from C7a recorded, known issues listed.

**Out of scope** (and which todo covers it)

- Publishing `@laminapdf/*` packages to npm (stays local; [architecture: Publishing (later)](../../plans/architecture.md#publishing-later)).
- Product naming if still open ([web-app §8 Q1](../../plans/web-app.md#8-open-questions)); the domain is `laminapdf.com`: the reader at `app.laminapdf.com`, the website at the apex.
- Accounts/login (hosted-service rounds after the `auth` spike).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Get the owner's statement of v0.1's pricing status (for example a free preview, or the free tier) and put it in the launch copy with the note that lamina will be a paid product.
- [ ] Size-budget script (`tools/` or `cargo xtask size web`) reading Vite's build manifest; min+gz each JS chunk loaded on startup (exclude lazy chunks only if they're truly lazy); CI step in the fast lane for PRs touching the same paths as C4d's deploy filter (`apps/web/`, `packages/`, `crates/`, lockfiles `pnpm-lock.yaml`/`Cargo.lock`, root manifests, and the workflow files) — reuse C4d's single filter list, so dependency bumps are size-checked.
- [ ] Promote C3c scenarios to gates in the release workflow (on the reference runner or a dedicated self-hosted/large runner) and add them to the `needs:` of C4d's tag-triggered production job; nightly keeps reporting and opening issues. If the reference run is manual (see Risks), the production job uses the `production` environment's required reviewer as the gate, and the reviewer approves only with the attached passing run.
- [ ] Lighthouse CI config (`lighthouserc.json`) with thresholds (accessibility ≥ 95, best practices ≥ 95, performance ≥ 90 on desktop profile; pinned `@lhci/cli` version, no `pwa` category) run against the PR preview (non-required) and release (required); add C7b's `installability.spec.ts` to the same preview job and to the release lane as a required step.
- [ ] Fix or waive outstanding perf issues from C3c/C5b/C7b (each waiver: issue link, owner, target version) — no silent misses.
- [ ] Write the privacy page content (plain language, reviewed against ADR 0011 point by point) and add it to the app; add a "Privacy" link in the UI and the README.
- [ ] `CHANGELOG.md` for `apps/web`; bump version; release notes (features, browsers, known issues, privacy summary).
- [ ] Run the full e2e suites (smoke, privacy, axe, installability) against this PR's preview deployment (C4d `BASE_URL`) and attach the results.
- [ ] After merge: tag `web-v0.1.0`; confirm production deploy, smoke, privacy and axe suites against production (Post-merge checks).
- [ ] Update plan statuses: [web-app.md](../../plans/web-app.md) status → Done (or In progress with remaining items), [roadmap milestones](../../plans/roadmap.md#milestones) M2 marked shipped, [architecture §7](../../plans/architecture.md#7-to-dos) Round C items ticked.

## Acceptance criteria

- [ ] The launch copy states v0.1's pricing status as approved by the owner, and that lamina will be a paid product (UI/UX principle 10).
- [ ] PR lane fails when the app shell exceeds 200 KB min+gz (demonstrated by a temporary test commit in the PR, then reverted).
- [ ] **Performance targets in §4 are met on the reference machine** (release-lane run attached to the PR) or each miss has an approved waiver issue.
- [ ] Lighthouse thresholds pass on this PR's preview deployment; C7b's installability spec passes against it.
- [ ] Smoke, axe and the privacy suite pass against this PR's preview deployment (production-equivalent build and headers).
- [ ] The privacy page is served at `/privacy` on the website's preview, linked from the app, and every statement maps to an ADR 0011 rule and to a test (C4c privacy suite) where testable.
- [ ] The build shows version `0.1.0` in the About/properties area (checked on the preview).
- [ ] All C1a–C7b todos are in `docs/todos/done/`.

## Post-merge checks

- [ ] Tag `web-v0.1.0` pushed on the squash-merge commit; the release lane (Lighthouse + installability + runtime perf gates) passes on it, and only then C4d's tag-triggered production job deploys.
- [ ] Production serves the tagged build: version `0.1.0` visible in the About/properties area.
- [ ] Smoke, privacy and axe suites pass against the production URL; `/privacy` is live on `laminapdf.com`, and the website links to the reader.
- [ ] Plan statuses updated after the release (web-app status, roadmap M2 shipped) in a follow-up PR if they weren't final at merge.

## Tests

Size-budget script unit tests; release-lane perf gates (C3c harness); Lighthouse CI; smoke + privacy + axe + installability suites against the PR preview (and, post-merge, against production).

## Docs to update

`apps/web/CHANGELOG.md`, `apps/web/README.md`, root `README.md` (link to the hosted reader and privacy page), `CLAUDE.md` (release/size commands), [web-app.md](../../plans/web-app.md) status and §9 ticks ("Performance budgets as CI gates", "Publish the privacy page; tag `web v0.1` and deploy to production"), [roadmap.md](../../plans/roadmap.md) M2, [architecture.md §7](../../plans/architecture.md#7-to-dos) Round C.

## Risks and open questions

- **Reference machine for runtime gates**: GitHub-hosted runners are noisy; a self-hosted runner or a fixed large runner is needed for gating; until then, gates run manually on the maintainer's reference machine and results are attached (decide with the owner).
- **Domain** is settled (the reader at `app.laminapdf.com`, the website at `laminapdf.com`; [§8](../../plans/web-app.md#8-open-questions) Q1); telemetry (Q3) is settled: none in v0.1.
- **Pricing status at launch**: lamina will be priced (owner decision 2026-10-29), but the pricing page ([U5a](../ready/U5a-pricing-and-promises-page.md)) follows the launch. So that later tiers aren't a changed deal ([UI/UX principle 10](../../plans/ui-ux.md#4-principles)), the owner states before launch what v0.1 is (for example a free preview, or the free tier) and the launch copy says that lamina will be a paid product.
- **Safari/iPadOS** issues found late may force waivers; budget a stabilisation week before tagging.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
