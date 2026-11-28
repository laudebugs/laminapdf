# E7k — Select and transform existing objects in the reader (A05)

| Field | Value |
| --- | --- |
| ID | `E7k` |
| Status | `done` |
| Plan | [E7 plan](../../plans/editor/e7-editing-existing-content.md) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6e](./E6e-content-tools-ui.md), [E7d](./E7d-select-transform-existing.md), [E7i](./E7i-structure-and-conformance-maintenance.md) |
| ADRs | Follows ADR 0022, 0024 |
| Branch | `feat/e7k-select-transform-ui` |
| PR title | `feat(ui): select, move and restyle existing objects in the reader` |
| Size | L (600–1,500) |
| PR | [#851](https://github.com/laudebugs/laminapdf/pull/851) |

## Goal

A05 in the web and desktop readers. With E6e's Select tool, people select the images, paths and text blocks the file already had. Then they move, resize, rotate, crop, reorder or delete them through E7d's `EditContent`, with the inspectors, context menu and states A05 draws.

## Context

- **Design (Paper):** [A05 Select and transform existing objects](../../design/paper/add-edit-content.md#a05-select-and-transform-existing-objects). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E7 plan](../../plans/editor/e7-editing-existing-content.md) §2 and §4
- [E6 plan §4](../../plans/editor/e6-adding-content.md#4-crate-and-package-changes): Edit mode's tools and handles, which this reuses

## Scope

**In scope**

- Hover outline and selection of existing objects through `hit_test`, groups by marquee and Shift
- E6e's handles for move, resize and rotate on existing objects; Crop on images
- The context menu (Bring forward and the rest), and the inspectors "Image", "n shapes" and "Text block"
- A05's states: a shared image, an object Lamina PDF can't change safely (outlined, no handles, the reason), a signature's appearance (read-only)
- "Add alternative text" in the Image inspector (A07): a `SetAlt { element, alt }` op (class `Document`, through the structure tree) that sets a figure's `/Alt`, so `figure-without-alt` can be fixed in Lamina PDF; until then its error page says to use an accessibility tool
- A07's strip, claim badges and Check before saving dialog ([A07](../../design/paper/add-edit-content.md#a07-structure-and-conformance-warning)), moved from [E7i](../done/E7i-structure-and-conformance-maintenance.md), which built the calls they use: `plan().conformanceWarnings` (`figure-without-alt`, `content-tags-broken`, `pdfa-font-not-embedded`) and `removeConformanceClaims` for Save without the claim

**Out of scope** (and which todo covers it)

- Edit points, paint, clips and masks → [E7h](../ready/E7h-vector-clipping-masks.md)
- Editing the text of a text block → [E7l](../ongoing/E7l-inline-text-editor.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.07`, `B.01`, `B.02`, `B.08`, `B.12`
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`, `apps/desktop`
- **`Op` variants:** `SetAlt { page, object, alt }` (class `Document`); the reader otherwise drives the core todo's ops. Deviation: the todo named it `SetAlt { element, alt }` and listed no new variant; the op names the picture the reader selected, and the core finds its figure, because the reader has no handle on structure elements
- **`SaveConstraint`:** as the core todo's: `Free` → incremental; a certified file becomes `Conflict` from the first edit, with the banner naming the certifier
- **Web vs desktop:** identical but for fonts on the computer (`systemFonts`, E6h)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.07`, `B.01`, `B.02`, `B.08`, `B.12` → `round-trip` (web, desktop)

## Tasks

- [x] Selection and hover
- [x] Handles and Crop
- [x] Context menu and inspectors
- [x] States
- [x] e2e

## Acceptance criteria

- [ ] Every A05 control works with a pointer and from the keyboard, and the command palette reaches every action. Not met in full: Cut, Copy, Duplicate, Edit points and the clipping and mask controls are E7h's or have no op; Crop and Add alternative text are in the palette (Crop image focuses the first inset), but there are no on-canvas crop handles
- [ ] Checked in the built reader in a browser, as a user does it (CLAUDE.md), and on the desktop app. The web reader was driven in Chromium by the new Playwright spec; the desktop app is not checked (the desktop half is a separate todo)
- [x] The reader's startup scripts don't grow beyond the budget (the viewer client's one new call and the status bar's badge slot are the only startup code; the rest loads with the editor's lazy chunk)
- [x] A07: an edit that breaks a claim shows the strip with the claim badges; Check before saving lists each claim; Save without the claim applies `removeConformanceClaims` for that claim only; Fix it first selects the figure and focuses Alternative text
- [x] Add alternative text sets the figure's `/Alt` through the new op, and `figure-without-alt` goes; the op meets full-editor §5's library parity, bar a facade example beyond the doctest

## As built

- **Reader** (`apps/web/src/content/`): `original.ts` (which objects Select lists, hits, marquee, the `EditContent` edits and restack bookkeeping; Node tests), `originals.ts` (the pick, drags, hover and every change as one op), `original-inspector.ts` (the "Image", "n shapes", "Text block" and "n objects" inspectors). The viewer client's `contentObjects(index)` reads what a page draws. The context menu, hover outline and marquee are in `controller.ts`, `mode.tsx` and `layer.ts`.
- **A07** (`packages/ui/src/edit/claims-ui.tsx`, `apps/web/src/save-flow.ts`): the strip, the status-bar badge and Check before saving, replacing E1c's "Save anyway" confirmation. Save without the claim applies `removeConformanceClaims` for the broken claims only, as one undo step, then saves; Fix it first selects the replaced picture.
- **`SetAlt { page, object, alt }`** (class `Document`): sets `/Alt` on the `Figure` that owns the object. Every refusal is `invalid-op`.
- **Deviations** are in the area document ([A05](../../design/paper/add-edit-content.md#a05-select-and-transform-existing-objects), [A07](../../design/paper/add-edit-content.md#a07-structure-and-conformance-warning)).

## Post-merge checks

None.

## Tests

- Playwright: move, resize, rotate, crop, reorder and delete an existing image and shape on the fixtures, then save, reopen and check
- WebdriverIO: the same on each desktop OS
- Playwright: replace a figure's image in a PDF/UA-1 file, see A07's strip and badges, add alternative text, and save with the claim; again, choosing Save without the claim, and check the saved XMP
- Unit: the alternative-text op on figures with and without `/Alt`, its undo and its frozen sample

## Docs to update

- User help for the screens this adds
- [E7 plan](../../plans/editor/e7-editing-existing-content.md) §7: point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan
- **Left for later** ([#852](https://github.com/laudebugs/laminapdf/issues/852)): the desktop half; Cut, Copy and Duplicate of existing objects (no op copies one); on-canvas crop handles; Apply to; Revert to original; the signature-appearance state, the entering-Edit-mode claims banner, the "still hold" toast and "Save a copy instead" for PDF/A; and splitting `controller.ts`, which forks on added and existing objects in many methods.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (not run yet: left for the PR's review)
- [ ] CI green (not waited for)
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
