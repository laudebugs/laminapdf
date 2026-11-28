# E4f — Comment mode: tools and the annotation inspector

| Field | Value |
| --- | --- |
| ID | `E4f` |
| Status | `done` |
| Plan | [E4 plan §4](../../plans/editor/e4-markup-and-drawing-annotations.md#4-crate-and-package-changes) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1e](./E1e-save-and-constraint-banner.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md), [E4c](./E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E4i](./E4i-viewer-annotation-layer.md) |
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
- One tool module per mark over [E4i](../done/E4i-viewer-annotation-layer.md)'s layer; text tools snap to text runs through the text layer's quads
- The inspector, with colours and widths remembered per tool on this device; the author name asked the first time a tool is chosen, kept on this device and changed from the inspector or the palette (its Settings entry is [E5g](../done/E5g-comments-panel.md)'s)
- Keyboard creation and editing (C01, C02), single-key shortcuts that Settings can turn off, ARIA names and focus order
- Resizing through the selection handles of E4i's layer and its `startGesture` (a spec patch per subtype): Shift keeps proportions, Alt+arrows resize from the bottom-right handle (WCAG 2.5.7), and each handle has a 24 px hit area (Paper C02)
- The banner from the moment Comment mode opens on a P1 or P2 file (C01); tools disabled with the reason on a file whose permissions forbid annotating
- The covered-text notice: a filled rectangle, ellipse or polygon at full opacity over text from the text layer gets a dismissible notice that the text stays in the file and can be copied, offering the redaction tool once [E12b](../ready/E12b-mark-and-review.md) exists; it never blocks the annotation

**Out of scope** (and which todo covers it)

- Text boxes and callouts → [E4j](../done/E4j-text-box-tools.md)
- The phone annotate bar and the tablet tool shelf → [E4k](../done/E4k-touch-and-stylus-annotating.md)
- Stamp and Attach file → [E5h](../done/E5h-stamps-and-attachments-ui.md); the Comments panel → [E5g](../done/E5g-comments-panel.md)
- The desktop → [E4h](../ready/E4h-desktop-annotations.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.01`, `D.02`, `D.03`, `D.04`, `D.05`, `B.06` in the interface
- **Crates and packages:** `laminapdf-model`, `laminapdf` (the façade), `laminapdf-wasm`, `@laminapdf/sdk` (a square's or circle's inner box in its record), `@laminapdf/viewer` (resizing in the annotation layer, `pageToUser`), `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** the banner and gating from E1e
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.01`, `D.02`, `D.03`, `D.04`, `D.05`, `B.06` → `round-trip` (web)

## Tasks

- [x] The mode and its toolbar
- [x] Tools
- [x] The inspector
- [x] The covered-text notice
- [x] Playwright specs per tool, tagged with capability IDs

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit: each tool makes its annotation with mouse and keyboard, saves, and the file reopened in pdf.js shows the same appearance and properties
- [ ] axe reports no serious violation in Comment mode, in three themes
- [ ] On the certified-P2 fixture the banner shows when the mode opens; on the P3 fixture there is none and a comment saves incrementally (the P3 fixture's page has no text to highlight, and pyHanko, which makes the fixtures, isn't set up here; every mark is the same class of op to the save planner)
- [ ] An opaque filled rectangle drawn over text shows the covered-text notice; a shape with no fill, a see-through fill, or one over empty space, shows none

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

## As built

- **The mode.** Comment registers after Read and before Organise, while a document is open (`apps/web/src/comment/mode.tsx`, loaded with later.ts). While it is the mode, the annotation layer sits over the pages, and its toolbar sits under the top bar: Select, Highlight, Underline, Strike, Caret, Note, Ink and Shapes ▾ (Rectangle, Ellipse, Line, Arrow, Polygon, Polyline, as radio items), then the tool's colour and width, or the page's comment count while Select is on. Text box, Stamp, Attach file and the Comments toggle stay with E4j, E5h and E5g. Every tool and inspector change is a command (`comment.*`, `annot.*`, `mode.comment`), so the palette reaches each one, and a tool chosen there switches to Comment first.
- **The tools.** Each mark has its own module (`apps/web/src/comment/tools.ts`): a state machine over page points, with Vitest cases. The text tools snap a drag to the page's glyph quads, keeping the glyphs between the press's line and the release's, so another column's lines stay unmarked. Caret goes on the glyph edge nearest the click. Ink strokes drawn within a second of each other make one Ink annotation. Polygon and Polyline end on a second click on the last corner, or on Enter. Shift draws squares, circles and 45° lines. Each mark is one `addAnnot`, drawn as a preview through `startGesture` and committed through it, so the preview goes in the frame the page's new pixels show. Its user-space geometry comes from the page's crop box, `/Rotate` and `/UserUnit` through `@laminapdf/viewer`'s `pageToUser`, which `fixtures/geometry/page-to-user.json` holds to laminapdf-model's `PageFrame::to_user_space`. Every op is built in `ops.ts`, typed as the SDK's `Op`, with one table of each annotation kind's facts.
- **The keyboard.** Enter with a text tool marks the selected text; choosing a text tool while text is selected marks it at once. Enter with any other tool places a default-size mark in the middle of the page in view, which the arrow keys then move. Escape goes back to Select. Delete removes the selection. The tools are one tab stop: the arrow keys move between them in the reading direction (swapped right to left), and Tab leaves for the colour and width. H, U and P are the single keys, which Settings turns off with the others. Rectangle has no `R`, since Reader mode holds it, and Arrow has no `A`, because a key can't carry Shapes' choice.
- **Resizing.** The layer gains a `resize` option (`can`, `ops`) in `@laminapdf/viewer`. With it, one selected annotation's handles each have a hit area 24 CSS px across; a drag resizes from the handle, Shift keeps the proportions (a flat line's box keeps its one side), and Alt+arrows resize from the handle at the bottom right of the screen, whatever the view's turn, 1 pt or 10 pt with Shift. `ops` may return a promise, so the app reads the page's frame before it writes the edit. `letPointerThrough` lets a drawing tool's pointer through the annotations, in place of a stylesheet rule. The layer stretches the record, and the app writes the subtype's spec patch: a square's or circle's box, a caret's box, ink strokes, a line's ends, a polygon's vertices. Text markups and notes don't resize. Each annotation's button is now at least 24 CSS px a side, which axe asked for on a caret.
- **A square's box, in Rust.** A square's or circle's `/Rect` holds its stroke too, by `/RD`, which the record didn't carry, so each resize would have grown the shape by its half-stroke. `AnnotGeometry::Inner` (laminapdf-model) and `AnnotationGeometry::Inner` (the façade), `{ kind: "inner", rect }` in TypeScript, now give the box the shape runs along. The changeset says so.
- **The inspector.** The inspector is docked in the shell's inspector slot while Comment has a selection. It shows the kind and page, then line colour or colour, fill, width, opacity, border (Solid, Dashed, Cloudy for closed shapes), line start and end, smoothing, the note, author and time, and Delete. Several selected annotations share colour, opacity and Delete. Placing a note focuses its note field. The record doesn't carry `/BS`, `/IC`, `/LE` or smoothing, so for an annotation the file brought, the inspector shows "As in the file" (or no swatch) and the file keeps its own values until they're changed. For what it writes in this session, the reader keeps those values in memory by annotation ID (`changes.ts`) and shows them. Adding them to `pageAnnotations` is [E4j](../done/E4j-text-box-tools.md)'s. A fill set from the inspector becomes that shape tool's fill, which is how a new shape is drawn filled; the toolbar has no fill control.
- **Colours, widths and the author.** Colours and widths are kept per tool on this device (`lamina.comment.styles`). The author's name is asked the first time a tool is chosen, kept on this device (`lamina.comment.author`) and changed from the inspector (Change your name…) or the palette (Your name on comments…). Its Settings entry is [E5g](../done/E5g-comments-panel.md)'s.
- **The banner.** On entering the mode, the controller asks `wouldViolate` about a comment. A P1 or P2 certification shows the banner (a region named "Signature warning") with Save a copy (Download a copy on the web, and the words name the same button) and Details; usage rights that refuse comments show it too. The planner names no signer, so the words leave the certifier's name out, and the status bar's "Certified" badge and the Signatures rail tab aren't built: all three are [E10j](../ready/E10j-signature-ui.md)'s. The only files whose permissions forbid annotating are encrypted ones, which can't take an edit until E11, so the tools are disabled with the session's own reason; the permission bit's gating and its test are [E11b](./E11b-encrypt-on-write.md)'s.
- **The covered-text notice.** After a shape is made or filled, the notice checks whether a filled rectangle, ellipse or polygon at full opacity covers the middle of a visible glyph. If it does, a dismissible notice under the toolbar says the text stays in the file. The bar's `data-cover-checks` counts the finished checks, so a test can wait for one before it looks for no notice. The offer of the redaction tool is [E12b](../ready/E12b-mark-and-review.md)'s.
- **Hints.** C01's one-line tool hint is a polite live region in the toolbar, not a status-bar slot, which the startup script owns. It and the page's count mount empty and take their text after, so a screen reader says the first one too, and the count says a page change only when the number changes.
- **Not built, and where it went.** Double-click for a one-shot tool and a test of the single-key switch: [E4j](../done/E4j-text-box-tools.md). Pen pressure: [E4k](../done/E4k-touch-and-stylus-annotating.md). Shift+F10's annotation menu, Reply and the selected annotation's full announcement: [E5g](../done/E5g-comments-panel.md). The user help page has no todo: the docs site has no reader help yet, as E2h found.
- **The acceptance criteria.** `apps/web/e2e/comment.e2e.test.ts` passes in Chromium, Firefox and WebKit. With the mouse and with the keyboard (Ellipse, Arrow and Polygon included), it makes every mark and saves; pdf.js reads each back with its appearance and author, and each kind's colour and, for drawings, its outline's width. It changes a rectangle in the inspector and resizes it with Alt+arrows. The banner shows on the certified-P2 fixture. On the P3 fixture there is none, and a note saves as an incremental update: its page has no text to highlight, and every mark is the same class of op to the planner. The covered-text notice shows for an opaque rectangle over text and not for an outline, a half-opaque fill or one over empty space; each "none" waits for the check to finish. axe runs on the mode, the inspector, the banner, the notice and the name question, in light, dark and high-contrast themes.
- **The startup budget.** The shell's inspector slot is filled in app.tsx from the edit session's hooks, and EditingRuntime, which loads later, renders inside the canvas, not into that slot, so the hook that puts Comment's inspector there stays in the startup script. With the commands' handlers it adds about 150 bytes to the startup script, which measured 287,268 bytes here against main's 287,117. The budget is now 287,770, by the standing rule.
- **Gallery screenshots** aren't refreshed by any workflow; the shared stylesheet gains `comment.css`, which no gallery route shows yet.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
