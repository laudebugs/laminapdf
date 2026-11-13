# E4f — Comment mode: tools and the annotation inspector

| Field | Value |
| --- | --- |
| ID | `E4f` |
| Status | `ready` |
| Plan | [E4 plan §4](../../plans/editor/e4-markup-and-drawing-annotations.md#4-crate-and-package-changes) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1e](../done/E1e-save-and-constraint-banner.md), [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4c](../ongoing/E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E4i](./E4i-viewer-annotation-layer.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e4f-annotation-tools-ui` |
| PR title | `feat(ui): Comment mode with its tools and the annotation inspector` |
| Size | L (400–800): the mode, nine tools over the annotation layer, the inspector and the covered-text notice; text boxes and touch are E4j and E4k |
| PR | — |

## Goal

Comment mode in the web reader: its secondary toolbar, the Select, Highlight, Underline, Strike, Caret, Note, Ink and Shapes tools, the colour and width controls, and the inspector for a selected annotation (colour, fill, width, opacity, border, line endings, smoothing, note), with the banner on files certified at P1 or P2.

## Context

- **Design (Paper):** [F02](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [C01 Comment tools](../../design/paper/comment-review.md#c01-comment-tools), [C02 Ink and shapes](../../design/paper/comment-review.md#c02-ink-and-shapes). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E4 plan §5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing) and [§6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- [ADR 0024](../../decisions/0024-app-shell-ux.md) §2: `mode.comment` owns one secondary toolbar
- [Hacker News research R2](../../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file): a box drawn over text is the classic failed redaction, and users ask editors to warn about it ([trust and proof plan §7](../../plans/trust-and-proof.md#7-changes-to-existing-todos-and-plans))

## Scope

**In scope**

- `mode.comment` in the mode registry and its toolbar; the `comment.*` tool commands and the `annot.*` inspector commands of C01 and C02, each in the command registry
- One tool module per mark over [E4i](E4i-viewer-annotation-layer.md)'s layer; text tools snap to text runs through the text layer's quads
- The inspector, with colours and widths remembered per tool on this device; the author name from Settings, asked the first time
- Keyboard creation and editing (C01, C02), single-key shortcuts that Settings can turn off, ARIA names and focus order
- The banner from the moment Comment mode opens on a P1 or P2 file (C01); tools disabled with the reason on a file whose permissions forbid annotating
- The covered-text notice: a filled rectangle, ellipse or polygon at full opacity over text from the text layer gets a dismissible notice that the text stays in the file and can be copied, offering the redaction tool once [E12b](E12b-mark-and-review.md) exists; it never blocks the annotation

**Out of scope** (and which todo covers it)

- Text boxes and callouts → [E4j](E4j-text-box-tools.md)
- The phone annotate bar and the tablet tool shelf → [E4k](E4k-touch-and-stylus-annotating.md)
- Stamp and Attach file → [E5h](E5h-stamps-and-attachments-ui.md); the Comments panel → [E5g](E5g-comments-panel.md)
- The desktop → [E4h](E4h-desktop-annotations.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.01`, `D.02`, `D.03`, `D.04`, `D.05`, `B.06` in the interface
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** the banner and gating from E1e
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.01`, `D.02`, `D.03`, `D.04`, `D.05`, `B.06` → `round-trip` (web)

## Tasks

- [ ] The mode and its toolbar
- [ ] Tools
- [ ] The inspector
- [ ] The covered-text notice
- [ ] Playwright specs per tool, tagged with capability IDs

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit: each tool makes its annotation with mouse and keyboard, saves, and the file reopened in pdf.js shows the same appearance and properties
- [ ] axe reports no serious violation in Comment mode, in three themes
- [ ] On the certified-P2 fixture the banner shows when the mode opens; on the P3 fixture there is none and a highlight saves incrementally
- [ ] An opaque filled rectangle drawn over text shows the covered-text notice; a shape with a transparent fill, or one over empty space, shows none

## Post-merge checks

None.

## Tests

- Playwright specs in `apps/web/e2e/`
- Vitest for the tool state machines

## Docs to update

- User help: annotating
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
