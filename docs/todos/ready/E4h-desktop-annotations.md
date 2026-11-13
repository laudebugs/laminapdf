# E4h — Desktop: markup and drawing annotations

| Field | Value |
| --- | --- |
| ID | `E4h` |
| Status | `ready` |
| Plan | [E4 plan §7](../../plans/editor/e4-markup-and-drawing-annotations.md#7-differences-between-the-shells) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [D4b](../ongoing/D4b-native-menus.md), [E1i](./E1i-desktop-save-in-place.md), [E2i](./E2i-signed-gating-and-close.md), [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4f](./E4f-annotation-tools-ui.md), [E4g](./E4g-appearance-parity-suite.md), [E4j](./E4j-text-box-tools.md), [E4k](./E4k-touch-and-stylus-annotating.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e4h-desktop-annotations` |
| PR title | `feat(desktop): Comment mode in the desktop app` |
| Size | M (200–400): the engine's calls, the native Comment menu and the WebdriverIO suites |
| PR | — |

## Goal

The desktop part of round E4 ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo), and the round's close. Comment mode runs over the engine's edit session, every tool is in the native Comment menu, and the signed-document table of the [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint) runs as one suite. With this todo merged, every E4 row's desktop column is complete.

## Context

- **Design (Paper):** [C01](../../design/paper/comment-review.md#c01-comment-tools) to [C03](../../design/paper/comment-review.md#c03-free-text-and-callouts), [C09](../../design/paper/comment-review.md#c09-tablet-stylus), desktop artboards
- [E4 plan §6 and §7](../../plans/editor/e4-markup-and-drawing-annotations.md#7-differences-between-the-shells)
- [E1h](E1h-desktop-edit-session.md), [E1i](E1i-desktop-save-in-place.md): the engine's edit session and saving in place
- [D4b](../ongoing/D4b-native-menus.md): the native menus

## Scope

**In scope**

- The engine runs the round's ops and `page_annotations`, and the annotation layer's hidden renders through its PDFium
- Every Comment tool in the native Comment menu (D4b)
- Pen tablets and Windows touch screens with E4k's pointer rules
- The signed-document suite: `would_violate` and the banner for every E4 op on each `fixtures/signed/` file, the usage-rights file included
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- System fonts for text boxes → [E6c](E6c-add-text.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): every E4 row's desktop column
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; the banner checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.01`, `D.02`, `D.03`, `D.04`, `D.05`, `D.06`, `D.16`, `B.06` → `round-trip` (desktop)

## Tasks

- [ ] The engine's calls for the round's ops and reads
- [ ] The native Comment menu
- [ ] The signed-document suite
- [ ] WebdriverIO specs `apps/desktop/e2e/specs/annotate.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: a highlight, a note, an ink stroke, a rectangle and a callout made through the interface, saved in place, reopened and checked
- [ ] On the certified-P3 fixture a highlight saves incrementally; on P2 the banner shows when Comment mode opens

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/annotate.e2e.ts`
- The signed-document suite in `crates/laminapdf-save/tests/`

## Docs to update

- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
