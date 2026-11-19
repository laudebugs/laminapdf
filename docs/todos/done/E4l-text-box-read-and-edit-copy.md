# E4l — Reading a text box back, and a text-capable edit copy

| Field | Value |
| --- | --- |
| ID | `E4l` |
| Status | `done` |
| Plan | [E4 plan §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4e](./E4e-freetext-callouts.md), [E4i](./E4i-viewer-annotation-layer.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0009](../../decisions/0009-web-client-architecture.md), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/e4l-text-box-read` |
| PR title | `feat(viewer): read a text box back, and edit in a text-capable copy` |
| Size | M (200–400) |
| PR | [#738](https://github.com/laudebugs/laminapdf/pull/738) |

## Goal

Two pieces [E4j](../done/E4j-text-box-tools.md) needs before it can edit text boxes in place, split off so its PR holds only the interface. First, a text box read back as it is: its text with bold and italic, font, size, alignment, fill, border, kind and callout line, whichever app made it, so an editor can open an existing box with what it holds and an inspector can show its values. Second, the viewer's worker edits in a copy that can lay out text: a document opened before the extract module loaded is edited in the base module, which refuses every text box (`appearance-needs-text`).

## Context

- [E4 plan §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated): the two tiers, standard 14 fonts in the extract module, fonts by their bytes and fallbacks in the text module; a document can't move between modules once open.
- [E4e](../done/E4e-freetext-callouts.md): `FreeText`, `FreeTextPatch` and the reading of `/DA`, `/DS` and `/RC` that `UpdateAnnot` already does (`laminapdf-ops`' `annot/freetext.rs`, `annot/redraw.rs`).
- `@laminapdf/viewer`'s worker (`src/worker/document.ts`): the text-capable copy, which replays every edit, undo and redo the model makes.

## Scope

**In scope**

- `laminapdf-ops`: `read_text_box(doc, id)`, the box as `UpdateAnnot` reads it, with its colour and whether reading dropped styles outside the subset.
- The façade's `Document::text_box(id)`: the box in page space beside its spec in user space; WASM `textBox(id)` (the extract and text modules, as `pageAnnotations` is); `@laminapdf/sdk`'s `doc.textBox(id)`, served by the worker client.
- The viewer: `client.textBox(id)`, and `client.editInTextCopy({ text })`, after which the worker edits in a copy that lays out text (the text module when asked), with the session's history kept.

**Out of scope** (and which todo covers it)

- The text tools, editing in place and the inspector → [E4j](../done/E4j-text-box-tools.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): none on their own; `D.06` in the interface is E4j's
- **Crates and packages:** `laminapdf-ops`, `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`, `@laminapdf/viewer`
- **`SaveConstraint`:** none (a read, and a copy that saves as the model did)
- **Web vs desktop:** the web's worker; the desktop's engine opens documents natively and needs only the read (E4h)
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [x] The read in `laminapdf-ops` and the façade, tested on Lamina PDF's boxes and another app's `/RC`
- [x] WASM, SDK and the worker client
- [x] The viewer's text-capable edit copy, tested with edits, undo and redo before and after the switch

## Acceptance criteria

- [x] A box Lamina PDF wrote reads back as the spec it was written from; a callout's line and a typewriter box keep their kind; an Acrobat-style `/RC` with a style outside the subset reads with `simplified`
- [x] In the worker, a document opened in the base module takes a text box after `editInTextCopy`, and undo then redo still walk the whole history

## Post-merge checks

None.

## Tests

- `laminapdf-ops` and façade tests; a WASM test in Node; `@laminapdf/sdk` Vitest; `@laminapdf/viewer` worker tests

## Docs to update

- `docs/agents/api-reading-and-editing.md`, `api-facade-and-cli.md` and `api-bindings.md`
- The E4 plan's todo table and [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)

## Risks and open questions

- The copy opens from the file's bytes as first opened; a document whose bytes were let go (a large local file read through OPFS keeps its source) has none, and `editInTextCopy` then says why.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file to `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
