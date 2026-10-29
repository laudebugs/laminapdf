# L6b — Mobile SDK plan: `LaminaView` (SwiftUI) and `LaminaViewer` (Compose), rounds and todos

| Field | Value |
| --- | --- |
| ID | `L6b` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L6](../../plans/shareable-libraries.md#l6-mobile), [§2 non-goals](../../plans/shareable-libraries.md#2-scope-and-non-goals) (the SDK rounds come from this plan) |
| Round / milestone | Round L6 (shareable libraries) — Mobile |
| Depends on | [L6a](./L6a-lamina-ffi-spike.md), [C2b](./C2b-lamina-render-crate.md), [U3b](./U3b-mobile-shell-spike.md) |
| Blocks | None |
| ADRs | Follows 0029 (from [L6a](./L6a-lamina-ffi-spike.md)), 0028 (jobs, cancellation), 0027 (coordinates), 0026 (errors) from [L1a](../done/L1a-contract-adrs.md), 0004 (render backend per surface, from [C2a](./C2a-render-adrs.md)), [0002](../../decisions/0002-binding-strategy.md), [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md); lists the ADRs the mobile rounds will need |
| Branch | `docs/l6b-mobile-sdk-plan` |
| PR title | `docs(plans): mobile SDK plan with SwiftUI and Compose viewers, rounds and todos` |
| Size | S–M (docs only: one plan plus its todo files) |
| PR | — |

## Goal

Write `docs/plans/mobile-sdk.md`: how lamina becomes a drop-in PDF SDK for iOS and Android that beats the free platform viewers on parity, fidelity and editing. It covers the SwiftUI `LaminaView` and Compose `LaminaViewer`, tiled rendering into caller-owned buffers, memory trimming, stylus ink written as `/Ink` with appearance streams, accessibility from the structure tree, forms, and packaging. The plan is split into rounds, and each round into todos on the board, so mobile work can start from `ready/` like every other round.

## Context

- [U3b](./U3b-mobile-shell-spike.md) and ADR 0025 (mobile shell): the SDK's views (`LaminaView`, `LaminaViewer`) and lamina's own mobile apps should share one view layer if ADR 0025 picks native UI; if it picks Tauri 2 mobile, the SDK views are a separate product. This plan states which.
- [Report: mobile SDKs must beat free platform viewers](../../research/libraries/shareable-libraries.md#mobile-sdks-must-beat-free-platform-viewers-on-parity-fidelity-and-editing): [the baseline moved](../../research/libraries/shareable-libraries.md#the-baseline-moved-free-viewers-now-annotate-and-fill-forms), [tiles and caller-owned pixels](../../research/libraries/shareable-libraries.md#tiles-a-page-size-table-and-caller-owned-pixels), [memory](../../research/libraries/shareable-libraries.md#memory-size-caches-from-the-live-budget-and-shed-on-warnings), [stylus](../../research/libraries/shareable-libraries.md#stylus-accept-platform-strokes-write-ink-with-appearance-streams), [concurrency](../../research/libraries/shareable-libraries.md#concurrency-generated-bindings-need-a-hand-written-idiomatic-layer), [packaging](../../research/libraries/shareable-libraries.md#packaging-spm-only-aars-on-maven-central); the SwiftUI, Compose and Rust API sketches.
- [Mobile SDK note](../../research/libraries/notes/mobile-sdk.md): [§1 platform and vendor APIs](../../research/libraries/notes/mobile-sdk.md#1-platform-and-vendor-api-surfaces-pdfkit-pdfrenderer-androidxpdf-nutrient-apryse-foxit-pdfium), [§2 tiles](../../research/libraries/notes/mobile-sdk.md#2-tile-based-virtualized-rendering), [§3 memory](../../research/libraries/notes/mobile-sdk.md#3-memory), [§4 stylus](../../research/libraries/notes/mobile-sdk.md#4-stylus-input-and-ink), [§7 accessibility, dark mode, RTL](../../research/libraries/notes/mobile-sdk.md#7-accessibility-dark-mode-rtl), [§8 complaints](../../research/libraries/notes/mobile-sdk.md#8-what-developers-complain-about-and-why-they-pay), [API sketch](../../research/libraries/notes/mobile-sdk.md#ideal-drop-in-api-sketch-recommendation-not-sourced).
- [Report: rendering per surface](../../research/libraries/shareable-libraries.md#rendering-hayro-is-the-small-footprint-candidate-pdfium-wins-on-fidelity-at-about-2-mb): PDFium's native weight on mobile vs hayro; ADR 0004 decides per surface.
- [L6a](./L6a-lamina-ffi-spike.md) results (spike README, sizes, cancellation findings) and ADR 0029.
- [C2b](./C2b-lamina-render-crate.md) (`RenderBackend`, tile requests, `max_tile_pixels`), [C3b](./C3b-viewer-virtualiser-and-zoom.md) (web virtualiser and zoom ideas to reuse), [E4d](./E4d-appearances-ink-and-shapes.md) (ink appearances), [full-editor E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and [E8](../../plans/full-editor.md#e8-form-filling-and-data) (annotation and form models the SDK reuses), [E10i](./E10i-platform-signers.md) (Secure Enclave and Android Keystore signers).
- [Research 03 §9.3](../../research/pdf/03-text-and-fonts.md#93-language-alt-text-reading-order) and [L3a](./L3a-lamina-extract-reading-order.md) (reading order for accessibility on untagged files).
- ISO 32000-2 §12.5.6.13 (ink annotations, `/InkList`), §12.5.5 (appearance streams), §12.2 (`/ViewerPreferences /Direction`), §14.7–14.9 (structure, alt text).

## Scope

**In scope**

- `docs/plans/mobile-sdk.md` following the shape of the other plans (goal, scope and non-goals, users, requirements, design summary, decisions to record, rounds, risks, open questions, to-dos), covering:
  - **Rust primitives** the SDKs sit on, as additions to the façade and `lamina-ffi`: `page_sizes()` at open; `render_tile(page, scale, rect, out, cancel)` into a caller-owned RGBA buffer; `trim(level)`; text and search as streams; annotation and form ops through the op log; incremental save.
  - **Views:** SwiftUI `LaminaView(document:state:)` with an `@Observable` state (page, zoom, selection, tool) and `.laminaTool`, `.onLaminaSave` modifiers; Compose `LaminaViewer(document, state, tool, formFillingEnabled)` with `rememberLaminaDocument` and `rememberLaminaViewerState`; headless use without views.
  - **Rendering:** low-resolution whole page first, then visible tiles on background threads; off-screen jobs cancelled; a page-size table so 1,000+ page lists lay out without rendering; Telephoto-style subsampling on Compose; CLI parity test (`lamina render --rect … --format rgba`) against device renders.
  - **Memory:** no decoded bitmaps kept in Rust by default; the platform owns pixel buffers and caches sized from the live budget (`os_proc_available_memory()` on iOS); `trim(level)` wired to `didReceiveMemoryWarningNotification` and `onTrimMemory`; object caches bounded by `Limits`.
  - **Stylus:** accept platform strokes (PencilKit over `PDFPageOverlayViewProvider`-style overlays on iOS, androidx.ink on Android); write `/InkList` plus an `/AP` appearance stream (pressure lives in the appearance); keep native stroke data under a private key for lossless re-editing; reuse E4d.
  - **Accessibility:** per-page accessibility elements from the structure tree, else from L3a reading order; VoiceOver and TalkBack; a reflow view because Dynamic Type cannot reflow fixed layouts; dark mode as a render-time colour filter; RTL from `/ViewerPreferences /Direction`.
  - **Forms:** filling through the E8 field model and appearances; platform autofill hooks; signature-safe saves (ADR 0007).
  - **Concurrency:** Swift `async` and `AsyncThrowingStream`, truly `Sendable` handles; Kotlin `suspend` and `Flow`, `Dispatchers.Default` for rendering; cancellation through `CancelToken` (ADR 0028) per L6a's findings.
  - **Packaging:** SPM only (binary XCFramework target + source wrapper target); Maven Central `lamina-core` and `lamina-compose` AARs; ABIs and 16 KB alignment per ADR 0029; size budgets per ABI; minimum OS versions; sample apps.
  - **Render backend on mobile:** PDFium vs hayro per ADR 0004's per-surface decision and C1a/C2b measurements.
  - **Competitive bar:** PDFKit, androidx.pdf, Nutrient; what lamina offers that they do not (identical behaviour on both platforms, tolerance, incremental save, undo, signing, redaction, generation, a non-experimental Compose API).
- **Rounds and todos:** split the plan into rounds, then write one todo file per PR in `docs/todos/ready/` (template format, Depends on links, Status `ready`). Expected rounds (recommendation, adjust in the PR): product `crates/lamina-ffi` and CI; render primitives and tiles; viewer core on each platform; text, search and accessibility; annotations and ink; forms; packaging and release.
- **Todo IDs.** Two options; recommend one in the plan and decide in the PR:
  1. Continue the L series (`L7a`, `L7b`, … `L12x`), keeping mobile under the shareable-libraries umbrella.
  2. A new letter for a mobile round series. **Recommendation:** a new letter, because the SDK work will outgrow this plan; avoid `M`, which the board already uses for milestones (M1–M9). Candidate: `S` (SDK) or `N` (native), checked against the board and `cargo xtask todos` before use.
- Add the plan to `docs/plans/README.md`, link it from [shareable-libraries §7 L6](../../plans/shareable-libraries.md#l6-mobile) and the [roadmap](../../plans/roadmap.md), and list new ADRs it proposes (numbers checked on the board first; e.g. accessibility mapping, mobile render backend if ADR 0004 leaves it open).

**Out of scope** (and which todo covers it)

- Any code, including the product `crates/lamina-ffi`: the first todo of the new plan.
- React Native, Flutter and Kotlin Multiplatform bindings: listed as later options with the ubrn/UniFFI pin constraint from ADR 0029.
- Desktop and web viewers: web plan ([C3b](./C3b-viewer-virtualiser-and-zoom.md)) and desktop plan.

## Tasks

- [ ] Read the Context links, the L6a spike README and ADR 0029; skim the [web plan](../../plans/web-app.md) and [full-editor plan](../../plans/full-editor.md) for the shape to follow
- [ ] **Verify before writing** (report items "to verify"): androidx.pdf release state (1.0.0-beta01 on 26 August 2026 vs 1.0.0-alpha19) on the English release-notes page, and whether its Compose `PdfViewer` is still `@ExperimentalPdfApi`; current PencilKit/PDFKit overlay APIs; record the dates checked
- [ ] Write `docs/plans/mobile-sdk.md` (Status Draft) with the sections above; label every recommendation; cite sources inline
- [ ] Decide the todo ID scheme (see Scope) and write the todo files, each with Depends on links to existing todos (C2b, E4d, E8*, E10i, L3a, L6a as relevant)
- [ ] Update `docs/plans/README.md`, [shareable-libraries §7 L6](../../plans/shareable-libraries.md#l6-mobile) and §9, the roadmap; regenerate the board (`cargo xtask todos`) and run `cargo xtask check docs`

## Acceptance criteria

- [ ] `docs/plans/mobile-sdk.md` exists, covers every topic in Scope, and lists its rounds with deliverables and "accepted when" criteria
- [ ] Every round has at least one todo in `docs/todos/ready/` in the template format; the board lists them and `cargo xtask todos --check` passes
- [ ] The androidx.pdf state and the other verified facts are stated with the date checked and a primary source
- [ ] The todo ID scheme is decided and written in the plan
- [ ] `cargo xtask docs-links` passes
- [ ] The plan is reviewed (PR approval) before merge

## Post-merge checks

None.

## Tests

None (docs only). `cargo xtask check docs` covers links and the board.

## Docs to update

- [ ] New `docs/plans/mobile-sdk.md`; `docs/plans/README.md` index
- [ ] [shareable-libraries §9 L6](../../plans/shareable-libraries.md#9-to-dos): tick "Mobile SDK plan with rounds and todos"
- [ ] [Roadmap](../../plans/roadmap.md): add the mobile rounds
- [ ] `docs/todos/README.md` (regenerated), `CLAUDE.md` if the new letter needs a mention

## Risks and open questions

- **androidx.pdf (to verify):** if beta01 is current and its Compose viewer stabilises soon, lamina's Android pitch shifts further to fidelity, editing and signature-safe saves; the plan should say so.
- **Render backend on mobile:** PDFium adds native weight (AndroidPdfViewer ships about 16 MB); hayro may suit mobile if C1a's fidelity holds. If ADR 0004 has not decided per surface by then, the plan lists it as a blocking open question.
- **Kotlin cancellation and Swift 6 Sendable:** carry L6a's findings forward; if UniFFI's behaviour is still unclear, the first SDK round includes a test that pins it.
- **Accessibility depth:** the research did not cover VoiceOver or TalkBack behaviour in detail; the plan should schedule a short research note before the accessibility round.
- **Scope creep:** the plan must stay a plan; anything that needs code to decide becomes a spike todo.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
