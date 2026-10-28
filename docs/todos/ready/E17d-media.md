# E17d — Media: Screen/Rendition, RichMedia and audio annotation playback

| Field | Value |
| --- | --- |
| ID | `E17d` |
| Status | `ready` |
| Plan | [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E5c](./E5c-stamps-and-attachments.md), [E17a](./E17a-advanced-interactions-feature-plan.md) |
| Blocks | [E17f](./E17f-3d-detection.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e17d-media` |
| PR title | `feat(media): embed and play audio and video` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Embed audio/video as Screen annotations with Rendition actions and RichMedia (PDF 2.0), and play them (desktop first; web where codecs allow), plus playback of Sound annotations.

## Context

- ISO 32000-2 §13 (multimedia)

## Scope

**In scope**

- Embedding ops
- Players per shell
- Poster appearances

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.05` embed audio and video; `M.11` rich-media annotations; `D.09` audio annotations
- **Crates and packages:** `lamina-model`, `@lamina/viewer`
- **`Op` variants:** `EmbedMedia` (class `Annotation`)
- **`SaveConstraint`:** annotation rules (see E4b): class `Annotation`, allowed under DocMDP P3 or when there is no certification signature (approval-only files save `AppendOnly`); under P1/P2 a violation → warn and gate
- **Web vs desktop:** desktop first; web limited to browser-supported codecs
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `M.05` → `round-trip` (core, pdf, web, desktop); `M.11` → `write` (core, pdf, web, desktop); `D.09` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Embedding
- [ ] Playback
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Embedded media plays in Acrobat (manual) and in lamina desktop

## Post-merge checks

None.

## Tests

- e2e

## Docs to update

- User help
- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
