# E6h — Fonts on the computer in the desktop font picker

| Field | Value |
| --- | --- |
| ID | `E6h` |
| Status | `ready` |
| Plan | [E6 plan §6](../../plans/editor/e6-adding-content.md#6-differences-between-the-shells) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [D2c](../done/D2c-engine-sandbox.md), [E6c](./E6c-add-text.md), [E6e](./E6e-content-tools-ui.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e6h-desktop-system-fonts` |
| PR title | `feat(desktop): offer the computer's fonts when adding text` |
| Size | M (200–600) |
| PR | — |

## Goal

On the desktop, the font picker lists the fonts on the computer after the bundled ones ("on this Mac", "on this PC"), and a chosen font's bytes reach the core, which embeds it. Split from [E6c](E6c-add-text.md) by the [E6 plan](../../plans/editor/e6-adding-content.md#7-todos).

## Context

- [A01 Add text box](../../design/paper/add-edit-content.md#a01-edit-mode-add-text-box) and the area's [fonts on web and desktop](../../design/paper/add-edit-content.md#shared-decisions-for-the-area)
- `laminapdf-desktop-sandbox`'s `SystemFonts` (D2c), which already lists and reads the faces the main process serves to the engine

## Scope

**In scope**

- A `systemFonts` capability flag and `Platform.fonts.list()` / `read(id)` (ADR 0010); the web platform has no such fonts
- The list from `SystemFonts`, with each face's family, style and `fsType`; a face whose licence forbids embedding is listed but disabled, with A01's reason
- Bytes passed to the core as a font blob

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.01` add text anywhere on a page; `A.12` insert special characters and symbols
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-sandbox`, `@laminapdf/ui` (platform)
- **`SaveConstraint`:** as E6c
- **Web vs desktop:** desktop only
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none (E6e flips the desktop columns)

## Tasks

- [ ] The capability flag and the platform calls (load `/tauri-v2`)
- [ ] The Tauri command and its capability
- [ ] The picker's sections
- [ ] e2e on each OS

## Acceptance criteria

- [ ] On macOS, Windows and Linux, a font on the computer can be picked, and the saved file embeds it
- [ ] A font that forbids embedding can't be picked

## Post-merge checks

None.

## Tests

- WebdriverIO on the three OSes; a unit test of the `fsType` check

## Docs to update

- `apps/desktop/README.md`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
