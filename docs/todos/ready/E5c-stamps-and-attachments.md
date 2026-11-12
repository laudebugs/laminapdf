# E5c — Stamps, file-attachment annotations and audio comments read

| Field | Value |
| --- | --- |
| ID | `E5c` |
| Status | `ready` |
| Plan | [E5 plan §2.1](../../plans/editor/e5-review-stamps-interchange.md#21-ops) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E3f](./E3f-attachments-and-associated-files.md), [E4b](./E4b-annotation-model-and-overlay.md), [E4e](./E4e-freetext-callouts.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5c-stamps-and-attachments` |
| PR title | `feat(build): stamps, file-attachment annotations and audio comments` |
| Size | L (400–800): two specs with their appearances, the Sound reader and WAV conversion |
| PR | — |

## Goal

Stamp annotations (the standard names, image stamps and dynamic text), FileAttachment annotations with their embedded files, and Sound annotations read and turned into WAV.

## Context

- [E5 plan §2.1 and §2.2](../../plans/editor/e5-review-stamps-interchange.md#21-ops)
- **Design (Paper):** [C05 Stamps and file attachments](../../design/paper/comment-review.md#c05-stamps-and-file-attachments)
- ISO 32000-2 §12.5.6.12 (stamps), §12.5.6.15 (file attachments), §12.5.6.16 (sound)

## Scope

**In scope**

- The `Stamp` spec: the standard names drawn by E4's generator, image stamps from the blob store, dynamic stamps laid out by `laminapdf-text` (`appearance-needs-text` in the base module)
- The `FileAttachment` spec: the file specification through [E3f](E3f-attachments-and-associated-files.md)'s writer, the icons of C05; `pdfa-attachment-forbidden` on PDF/A-1 and -2
- `page_annotations` records for Stamp, FileAttachment and Sound; `sound_as_wav(id)` for raw, signed, μ-law and A-law samples within `Limits`
- `fixtures/annotations/stamps.pdf`, `attachments.pdf`, `sound.pdf`

**Out of scope** (and which todo covers it)

- The stamp picker, custom stamps, the inspector and Play → [E5h](E5h-stamps-and-attachments-ui.md)
- Recording audio → [E17d](E17d-media.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.07` custom stamps; `D.08` attach files as annotations; `D.09` audio annotations
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf-build`, `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** `AddAnnot` with `Stamp` and `FileAttachment` (class `Annotation`)
- **`SaveConstraint`:** as [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.07`, `D.08` → `round-trip` (core, pdf); `D.09` → `read` (core, pdf)

## Tasks

- [ ] Stamp generators and goldens
- [ ] The FileAttachment spec
- [ ] The Sound reader and `sound_as_wav`
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new op and function is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdicts of [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [ ] Each stamp renders in pdf.js and PDFium within E4g's threshold
- [ ] pdf.js's `getAttachments` returns the attached file's exact bytes
- [ ] `sound_as_wav` gives a playable WAV for each sample format of `sound.pdf`

## Post-merge checks

None.

## Tests

- Goldens in `fixtures/golden/annots/stamps.pdf` and render diffs
- The round-trip suite

## Docs to update

- API docs: stamps, file attachments and audio
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
