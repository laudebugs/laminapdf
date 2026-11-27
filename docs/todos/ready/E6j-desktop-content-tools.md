# E6j — Edit mode's content tools on the desktop

| Field | Value |
| --- | --- |
| ID | `E6j` |
| Status | `ready` |
| Plan | [E6 plan §6](../../plans/editor/e6-adding-content.md#6-differences-between-the-shells) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6e](../done/E6e-content-tools-ui.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e6j-desktop-content-tools` |
| PR title | `feat(desktop): add text, images and shapes in Edit mode` |
| Size | M (200–600) |
| PR | — |

## Goal

The desktop reader gets the Edit mode that [E6e](../done/E6e-content-tools-ui.md) built for the web: the same tools, handles and inspector over the desktop engine, and the OS clipboard and Finder or Explorer drag for pictures. E6e left the desktop out because the desktop has its own engine client, so the viewer client's new `addedObjects` call has to cross the engine protocol first.

## Context

- [E6e](../done/E6e-content-tools-ui.md) and [A01](../../design/paper/add-edit-content.md#a01-edit-mode-add-text-box), [A02](../../design/paper/add-edit-content.md#a02-place-image-and-shapes): the mode, its controller and layer are in `apps/web/src/content/` and `@laminapdf/ui`
- `apps/desktop/src/engine/desktop-engine-client.ts`: the desktop's implementation of the viewer client, which has no `addedObjects` yet
- [E6 plan §6](../../plans/editor/e6-adding-content.md#6-differences-between-the-shells): images in from the OS clipboard and drag from the file manager

## Scope

**In scope**

- `addedObjects(index)` in the engine protocol and the desktop engine client, answered by the engine's document
- Mounting Edit mode in the desktop app, from the code the web reader shares
- Picture paste from the OS clipboard and drag from Finder or Explorer; pictures the OS can decode but the core can't (HEIC and others) converted to PNG at the edge
- The desktop columns of the capability rows E6e flips on the web
- WebdriverIO scenarios for each tool on the three OSes

**Out of scope** (and which todo covers it)

- Fonts on the computer in the picker ([E6h](E6h-desktop-system-fonts.md))

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.01`, `A.12`, `B.01`, `B.02`, `B.04`, `B.05`, `B.09`
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-protocol`, `laminapdf-desktop-engine`
- **`SaveConstraint`:** as E6b–E6c
- **Web vs desktop:** desktop only
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.01` → `round-trip` (desktop); `A.12` → `round-trip` (desktop); `B.01` → `write` (desktop); `B.02` → `write` (desktop); `B.04` → `round-trip` (desktop); `B.05` → `round-trip` (desktop); `B.09` → `round-trip` (desktop)

## Tasks

- [ ] `addedObjects` in the protocol, the engine and the client
- [ ] Edit mode in the desktop app
- [ ] Clipboard and drag-in pictures, with conversion
- [ ] WebdriverIO scenarios

## Acceptance criteria

- [ ] e2e on macOS, Windows and Linux for every tool
- [ ] A picture pasted from the OS clipboard and one dragged from the file manager are placed

## Post-merge checks

None.

## Tests

- WebdriverIO; a protocol round-trip test for `addedObjects`

## Docs to update

- `apps/desktop/README.md`
- `docs/capabilities/README.md` through `cargo xtask capabilities`

## Risks and open questions

- The engine reads the objects from the same document the edits change; the web worker reads them from the edit model, and the desktop engine must too, not from a reopened copy
