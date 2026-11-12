# E3c — Internal links, external hyperlinks and links on text

| Field | Value |
| --- | --- |
| ID | `E3c` |
| Status | `ready` |
| Plan | [E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [C5b](../done/C5b-find-in-document.md), [E2j](./E2j-page-tree-and-tree-writers.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3c-links` |
| PR title | `feat(ops): create and edit link annotations` |
| Size | M (200–400): three ops, the structure-tree placement and the link tool |
| PR | — |

## Goal

Create, edit and delete Link annotations: internal (GoTo/destination), external (URI), on a drawn rectangle or on selected text (QuadPoints from glyph quads), with a link tool and external-link confirmation preserved from C6.

## Context

- **Design (Paper):** [O12 Link editor](../../design/paper/pages-navigation-extraction.md#o12-link-editor). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §1.4 Link annotations](../../research/pdf/04-interactive-security.md#14-link-annotations-12565)
- [04 §1.3 Actions](../../research/pdf/04-interactive-security.md#13-actions-126)
- [web C5](../../plans/web-app.md#c5-text-selection-and-search): glyph quads for text selection

## Scope

**In scope**

- Ops: `AddLink { id, nm, page, rect, quads, target, contents }` (user-space rectangle and quads, [E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops)), `UpdateLink`, `DeleteLink` with `/NM` = ID. They are the link-specific ops for good (frozen on the wire in `fixtures/ops/v1/`): when [E4b](E4b-annotation-model-and-overlay.md) adds the generic annotation core, these three become thin wrappers over it (same undo records, same IDs, same `/NM` and `/P` handling), and E4b's `AddAnnot` has no `Link` spec, so there is exactly one way to create a link
- Link tool in `@laminapdf/ui`: draw a rect or make a link from the current text selection; edit target in a popover; `/Contents` description (accessibility, overlaps L.07)
- URI validation and scheme allow-list (http, https, mailto); anything else fails with `link-uri-refused`
- Tagged files: the link joins the structure tree as a `Link` element with an `OBJR`, `/StructParent` and a `/ParentTree` entry, through [E2j](E2j-page-tree-and-tree-writers.md)'s number-tree writer; `link-not-tagged` when it can't be placed on a PDF/UA-1 file

**Out of scope** (and which todo covers it)

- The desktop column → [E3k](E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.02` internal page links; `I.03` external hyperlinks; `A.09` add hyperlinks to text
- **Crates and packages:** `laminapdf-model` (Link annotation view), `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** `AddLink`, `UpdateLink`, `DeleteLink` (class `Annotation`)
- **`SaveConstraint`:** `Free` → incremental. Signed: link annotations are class `Annotation`, allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations, saved `AppendOnly`); under P1/P2 a violation → warn and gate (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.02` → `round-trip` (core, pdf, web); `I.03` → `round-trip` (core, pdf, web); `A.09` → `round-trip` (core, pdf, web)

## Tasks

- [ ] Link view and ops
- [ ] Quads from selection
- [ ] Tool and popover
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Links created in lamina navigate correctly in pdf.js and PDFium (e2e clicks in pdf.js viewer harness)
- [ ] veraPDF reports nothing new on `golden/tagged-report.pdf` after adding a link on its text
- [ ] A certified-P3 file saves a new link incrementally; P1 and P2 refuse it before it applies

## Post-merge checks

None.

## Tests

- Unit, round-trip, e2e

## Docs to update

- API docs: links
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- `javascript:` URIs must be rejected on creation and flagged on read

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
