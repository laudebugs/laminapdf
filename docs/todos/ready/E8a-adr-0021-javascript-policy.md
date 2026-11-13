# E8a — ADR 0021: PDF JavaScript policy

| Field | Value |
| --- | --- |
| ID | `E8a` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E7b](../ongoing/E7b-editing-existing-content-feature-plan.md) |
| ADRs | Writes ADR 0021 |
| Branch | `docs/e8a-adr-0021-javascript-policy` |
| PR title | `docs(adr): 0021 PDF JavaScript policy` |
| Size | M (200–600) |
| PR | — |

## Goal

Decide what lamina does with PDF JavaScript before forms ship (full-editor §8: "ADR 0021 before E8"): never execute vs a restricted sandbox (e.g. QuickJS-in-WASM with only the Acrobat form-helper API: `AFNumber_Format`, `AFDate_FormatEx`, `AFSimple_Calculate`, `event`, `getField`) for formatting, validation and calculation; how JS is detected, reported and stripped; and what authoring (E17b) may write.

## Context

- [04 §4 JavaScript in PDF](../../research/pdf/04-interactive-security.md#4-javascript-in-pdf): locations, API, security history (CVE-2024-4367)
- [full-editor §9](../../plans/full-editor.md#9-open-questions) Q1
- [04 §3.3 Calculation order and scripts](../../research/pdf/04-interactive-security.md#33-calculation-order-and-scripts)
- [06 §3 group E/M](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): E.05, E.07, E.08, E.10, M.01

## Scope

**In scope**

- ADR 0021 file with options: (1) never execute, native AF* re-implementations only for recognised patterns; (2) sandboxed engine opt-in; (3) both
- Threat model (no I/O, no timers beyond budget, no network, CPU/memory caps, deterministic)
- Licence and size check of candidate engines (QuickJS: MIT; Boa: MIT/Unlicense; rquickjs) — no GPL
- JS inventory requirement: every location listed in the UI
- Index updates

**Out of scope** (and which todo covers it)

- Implementation of native AF formatting (E8f) and JS action authoring (E17b)

**Editor facts**

- **Crates and packages:** (decision) possible new edge crate `lamina-js` if a sandbox is chosen
- **`SaveConstraint`:** no new save behaviour beyond the round's: incremental; filling allowed without certification and under DocMDP P2/P3 (not P1), blocked for fields locked by FieldMDP/`/Lock`
- **Web vs desktop:** must state whether a sandbox runs on web, desktop or both
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Survey pdf.js's QuickJS sandbox and PDFium's FXJS scope
- [ ] Measure engine wasm size
- [ ] Write the ADR
- [ ] Owner sign-off

## Acceptance criteria

- [ ] ADR accepted with a decision for E8 (formatting/validation/calculation) and E17 (authoring)
- [ ] Security review notes attached

## Post-merge checks

None.

## Tests

None (decision).

## Docs to update

- `docs/decisions/README.md`
- architecture §3 table
- `CLAUDE.md` ADR list
- full-editor §9 Q1 answered
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Running any JS from documents is a large attack surface; if chosen, the sandbox needs fuzzing and an external review before release

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
