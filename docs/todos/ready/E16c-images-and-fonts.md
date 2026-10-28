# E16c — Downsample and recompress images; embed or subset fonts

| Field | Value |
| --- | --- |
| ID | `E16c` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [E16b](./E16b-structural-optimiser.md) |
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

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Text extraction unchanged after subsetting
- [ ] Image DPI targets met

## Tests

- Corpus

## Docs to update

- User help
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
