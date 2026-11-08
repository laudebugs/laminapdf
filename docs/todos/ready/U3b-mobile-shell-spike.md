# U3b — `mobile-shell` spike: Tauri 2 mobile vs UniFFI with native UI

| Field | Value |
| --- | --- |
| ID | `U3b` |
| Status | `ready` |
| Plan | [UI/UX §6 U3. Mobile and touch](../../plans/ui-ux.md#u3-mobile-and-touch) |
| Round / milestone | Round U (UI/UX) — after M2; answers architecture open question 4 |
| Depends on | [C2b](../done/C2b-lamina-render-crate.md), [U3a](../ongoing/U3a-responsive-mobile-and-touch.md) |
| ADRs | Informs 0025 (mobile shell, proposed by this spike); follows 0001, 0002 (UniFFI later), 0004 |
| Branch | `spike/u3b-mobile-shell` |
| PR title | `spike(mobile): compare tauri 2 mobile with uniffi and native ui` |
| Size | M (throwaway spike code outside the workspaces; README with measurements) |
| PR | — |

## Goal

Decide how native iOS and Android apps would be built before anyone builds one. The engine side is low risk (core crates already check for iOS and Android; PDFium binaries exist for both), so the spike measures UI shell fidelity: selection-menu conflicts, Apple Pencil latency, share sheet and Files integration, and on-device OCR access.

## Context

- [UI/UX plan §5.4](../../plans/ui-ux.md#54-mobile-and-touch-r17); [research: mobile](../../research/product/pdf-app-product-and-ux.md#mobile-stays-an-open-question-with-a-clear-spike).
- [Architecture §8 Q4](../../plans/architecture.md#8-open-questions-for-the-team) — Tauri mobile vs UniFFI with native UIs.
- [ADR 0002](../../decisions/0002-binding-strategy.md) — UniFFI is the planned native binding.
- [Differentiation notes](../../research/product/notes/differentiation-mvp.md) — Apple Vision `RecognizeDocumentsRequest`, ML Kit v2.
- Load the `/tauri-v2` skill before the Tauri half (CLAUDE.md).

## Scope

**In scope**

- `spikes/mobile-shell/tauri/`: Tauri 2 mobile app reusing `@laminapdf/ui` + `@laminapdf/viewer` with a native render path; iOS and Android builds.
- `spikes/mobile-shell/native/`: a minimal SwiftUI app and a minimal Compose app calling the core through UniFFI and rendering pages with PDFium.
- Measurements on the same devices: cold start, first page, scroll smoothness (dropped frames), memory on a 1,000-page file, pen-to-ink latency (high-speed video or system tracing), native selection menu behaviour over the page, share-sheet open, Files/Storage Access Framework open and save, Vision / ML Kit OCR call from each shell, app size.
- A README with the table, a recommendation, and a draft of ADR 0025.

**Out of scope** (and which todo covers it)

- Building the mobile apps → a new plan after ADR 0025 is accepted.
- Store publishing, signing for mobile → that plan.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Pick test devices (one recent and one older iPhone, one mid-range Android) and fixtures (text, scanned, vector-heavy, 1,000 pages).
- [ ] Build both shells to the same feature level: open from share sheet, render, scroll, zoom, select text, draw a stroke, run OCR on a scanned page.
- [ ] Measure and record each criterion with method notes.
- [ ] Write the recommendation and the ADR 0025 draft; add the spike to [architecture §4](../../plans/architecture.md#4-spikes-to-close-open-questions).

## Acceptance criteria

- [ ] The README answers all four criteria (selection menu, pen latency, share/Files, OCR access) for both shells with measurements or recorded observations.
- [ ] Both shells build reproducibly from the README's commands.
- [ ] A recommendation and an ADR 0025 draft exist; architecture open question 4 links to them.

## Post-merge checks

None.

## Tests

None beyond what the spike needs (spikes are outside CI, like `krilla-fit`).

## Docs to update

`spikes/mobile-shell/README.md`; [architecture §4 and §8](../../plans/architecture.md#4-spikes-to-close-open-questions); tick the spike item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Tauri mobile's maturity moves fast: record the versions and date every measurement.
- Device access for both platforms: the owner may need to provide devices or a device cloud with a permissive account.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
