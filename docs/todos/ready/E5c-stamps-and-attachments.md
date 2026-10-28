# E5c — Custom stamps, file-attachment annotations and audio annotations (read)

| Field | Value |
| --- | --- |
| ID | `E5c` |
| Status | `ready` |
| Plan | [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E3f](./E3f-attachments-and-associated-files.md), [E4b](./E4b-annotation-model-and-overlay.md), [E5a](./E5a-review-stamps-interchange-feature-plan.md) |
| Blocks | [E5d](./E5d-xfdf-fdf.md), [E10c](./E10c-visual-signatures.md), [E17d](./E17d-media.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5c-stamps-and-attachments` |
| PR title | `feat(annots): custom stamps and file attachment annotations` |
| Size | M (200–600) |
| PR | — |

## Goal

Stamp annotations with image or dynamic-text appearances ("Approved by {user} on {date}" resolved at creation, not by JavaScript), a stamp library stored per user on the device, FileAttachment annotations (embedded file spec + icon), and Sound annotations listed and playable on desktop (read).

## Context

- [04 §2.2](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo)
- [04 §6.1 File specifications](../../research/pdf/04-interactive-security.md#61-file-specifications-711)

## Scope

**In scope**

- Stamp builder (image via `lamina-build` images, text via lamina-text)
- Stamp library UI
- FileAttachment create/extract
- Sound annotation listing; desktop playback via native audio decoding, web lists only

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.07` custom stamps; `D.08` attach files as annotations; `D.09` audio annotations
- **Crates and packages:** `lamina-build`, `lamina-model`, `@lamina/ui`
- **`Op` variants:** `AddAnnot` specs `Stamp`, `FileAttachment`
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** stamp library: desktop app data folder, web IndexedDB (per viewer); audio playback on desktop only
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.07` → `round-trip` (core, pdf, web, desktop); `D.08` → `round-trip` (core, pdf, web, desktop); `D.09` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Builders
- [ ] Library storage per shell
- [ ] Attachment annotation
- [ ] Sound listing/playback

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Stamps render identically in pdf.js and PDFium; attached file extracts byte-identical

## Post-merge checks

None.

## Tests

- Goldens, round-trip, e2e

## Docs to update

- User help: stamps
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
