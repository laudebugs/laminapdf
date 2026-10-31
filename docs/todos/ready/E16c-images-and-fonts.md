# E16c — Downsample and recompress images; embed or subset fonts

| Field | Value |
| --- | --- |
| ID | `E16c` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [B4b](../done/B4b-shaping-bidi-line-breaking.md), [B4c](../done/B4c-standard-14-and-simple-fonts.md), [E16b](./E16b-structural-optimiser.md) |
| Blocks | [E16e](./E16e-preflight-and-conversion.md) |
| Dependency notes | [E16b](E16b-structural-optimiser.md), `B4` ([pdf-library §B4](../../plans/pdf-library.md#b4-fonts-and-text): fonts, shaping, subsetting) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16c-images-and-fonts` |
| PR title | `feat(optimise): image recompression and font embedding/subsetting` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Downsample/recompress images (JPEG quality, target DPI, keep masks, skip JPX unless decodable) and embed missing fonts or subset embedded ones (glyph closure from used glyphs).

## Context

- **Design (Paper):** [X09 Optimise](../../design/paper/convert-compare-produce.md#x09-optimise). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §6 Font embedding & subsetting](../../research/pdf/03-text-and-fonts.md#6-font-embedding--subsetting-from-code)

## Scope

**In scope**

- Image pipeline
- Font embedding (desktop system fonts / supplied fonts) and subsetting of existing fonts

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `K.02` downsample and recompress images; `K.03` embed or subset fonts
- **Crates and packages:** `lamina-save`, `lamina-text`, codecs
- **`SaveConstraint`:** `RequiresRewrite` for optimisation (full rewrite); signed files → `Conflict`, warned and overridden only explicitly
- **Web vs desktop:** desktop can embed system fonts; web only supplied fonts
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `K.02` → `round-trip` (core, pdf, web, desktop); `K.03` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Images
- [ ] Fonts
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Text extraction unchanged after subsetting
- [ ] Image DPI targets met

## Post-merge checks

None.

## Tests

- Corpus

## Docs to update

- User help
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
