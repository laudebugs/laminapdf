# E2i — DocMDP gating tests on signed fixtures and E2 round close

| Field | Value |
| --- | --- |
| ID | `E2i` |
| Status | `done` |
| Plan | [E2 page organisation §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2g](./E2g-page-labels.md), [E2m](./E2m-document-dialogs.md), [E2o](./E2o-structure-tree-merge.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `test/e2i-signed-gating-and-close` |
| PR title | `test(editor): signed-document gating suite for page operations` |
| Size | M (200–400): a table-driven test over every op and signed fixture, the cross-op labels test, one e2e test and the fixture script; no product code |
| PR | [#682](https://github.com/laudebugs/laminapdf/pull/682) |

## Goal

Prove, across all E2 ops, that signed documents are protected exactly as ADR 0007 §2 says: every page op on certified (P1/P2/P3) fixtures is blocked or requires the named-signature override, and the overridden save produces a document whose broken signatures were removed (§3); on approval-only fixtures the same ops save `AppendOnly` as an incremental update that leaves the signature intact, unless they change a field locked by FieldMDP/`/Lock`; then verify the round's capability rows and close the round's web and shared columns (the desktop column is [E2n](../ready/E2n-desktop-organise.md)'s).

## Context

- **Design (Paper):** [O03 Close with unsaved changes](../../design/paper/edit-session-organise.md#o03-close-with-unsaved-changes). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [ADR 0007](../../decisions/0007-save-planner.md) §2–§3
- [04 §8.6 Certification vs approval; MDP](../../research/pdf/04-interactive-security.md#86-certification-vs-approval-signatures-mdp)
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) save rules

## Scope

**In scope**

- Extend the `fixtures/signed/` set from E1c with signed documents that have outlines, named destinations, forms and tagged structure (so page ops have something to break)
- Table-driven test over every E2 op × fixture asserting `would_violate`, banner kind and override behaviour, row by row against the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) table (signed widgets, FieldMDP locks per field, UR3)
- The fixtures of the [E2 plan §9](../../plans/editor/e2-page-organisation.md#9-fixtures): approval-signed with a visible widget on page 2, an outline, a form and tags; certified P2 with an outline; a FieldMDP lock whose widget is on page 1; a usage-rights file whose `/Perms /UR3` is written by the script (it can't be validly signed without Adobe's key; the planner reads only its structure)
- Validation of overridden outputs: pyHanko/`pdfsig` show no broken signatures, fields remain unsigned
- Capability-matrix check: every E2 row at its target status in the core, pdf and web columns
- With the usage-rights file: the annotation ops' rights checked per op in `would_violate` (`/Create` for `AddAnnot`, `/Modify` for `UpdateAnnot`, `/Delete` for `DeleteAnnot`), which E4c left for this fixture ([E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint))

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** tests only (`crates/laminapdf/tests/signed_gating.rs`, e2e spec)
- **`SaveConstraint`:** exercises all kinds
- **Web vs desktop:** e2e checks the banner on the web; the desktop's is in [E2n](../ready/E2n-desktop-organise.md)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** verifies (does not newly flip) E2 rows; fixes any row still short of target

## Tasks

- [x] Extend the fixture generation script and README provenance
- [x] Table-driven Rust test
- [x] Cross-op labels test: after `SetPageLabels` (roman front matter, arabic body, a prefixed appendix), each of `InsertBlankPages`, `DeletePages`, `DuplicatePages`, `MovePages`, `ReversePages`, `InsertPagesFrom`, `ReplacePages`, extract and merge keeps every surviving page's label, read back in pdf.js and PDFium (moved here from E2g, the first todo after every one of those ops)
- [x] One Playwright e2e for the banner
- [x] Run `cargo xtask capabilities` and fix gaps
- [x] Tick E2 to-dos in the plan

## Acceptance criteria

- [x] Every E2 op × signed fixture case has an asserted verdict: violation under P1, P2, P3 and UR3 (`SetPageLabels` and the `PageStructure` ops alike); `AppendOnly` with no violation on the approval-only fixture, except `Restriction::SignedWidget` for the ops the plan's §6 lists on the signed widget's page; violation on the FieldMDP-lock fixture only for the ops §6 lists on a page holding a locked field's widget
- [x] Approval-only fixture: each E2 op's incremental save validates in pyHanko with the original signature intact (its `/ByteRange` unchanged) and the later revision reported as a modification
- [x] Overridden saves validate as unsigned (no "broken" signatures) in pyHanko and `pdfsig`
- [x] Labels match `PageLabels::remap`'s expectation after every op in the cross-op test (no page relabelled except new ones)
- [x] All E2 rows at target status in the core, pdf and web columns

## As built

- **Fixtures.** `fixtures/signed/make.py` writes four more: `approval-organised.pdf` (three pages, an outline, a text field, tags, the signature's widget on page 2), `certified-p2-outline.pdf`, `locked-page1.pdf` (a FieldMDP `/Include` lock on the field on page 1, the signature on page 3) and `usage-rights.pdf` (`/Perms /UR3` granting `/Annots [/Create /Delete]` and `/Form [/FillIn]`, written with no valid signature). Rows in `fixtures/README.md`.
- **The gating suite** (`crates/laminapdf/tests/signed_gating.rs`) runs the 14 E2 ops on every page of eight signed fixtures:
  - `would_violate` before the op, and the planner's verdict and constraint once it applies, each against the plan's §6;
  - an incremental save of every allowed op on the approval files, whose bytes start with the signed file;
  - the override on every refused op, after which no signature is left.
- **pyHanko and `pdfsig`.** `fixtures/signed/check_saved.py` validates the files the suite saves with `LAMINA_SIGNED_GATING_OUT` set. Run by hand (pyHanko 0.38.0, `pdfsig` 26.10.0), all 212 passed:
  - the 68 allowed saves: each signature intact, covering its whole revision, with the later revision reported as a modification (`ModificationLevel.OTHER`); `pdfsig` calls each valid;
  - the 144 overridden saves: pyHanko finds no signature, and `pdfsig` lists the field as unsigned.

  CI doesn't run the script, so `pdfsig` (GPL) stays an outside tool. The suite's own checks run in CI: the signed bytes unchanged, and no signature after the override.
- **Usage rights per op.** E4c left this for the usage-rights file. Each annotation op now names the right it needs (`Op::annot_right`: `/Create`, `/Modify` or `/Delete`), and pending changes record them (`PendingChanges::annot_rights`). The planner checks each right against `/Annots`, both in `would_violate` and at save time; ADR 0007 §9 records it.
- **Cross-op labels** (`crates/laminapdf-save/tests/cross_op_labels.rs`). `SetPageLabels` gives `pages/labels.pdf` the labels i–iii, 1–4 and A-1 to A-3, then nine edits each run once: insert blank pages, delete, duplicate, move, reverse, insert from a file, replace, merge and extract. Every surviving page keeps its label.
  - The labels match `PageLabels::remap`, except after a merge: its copies keep their source's labels (`MergeDocuments`), so `hello.pdf`'s page is 1, not A-4.
  - The saved files are in `fixtures/pages/cross-op-labels/`. Their facts (`tools/fixtures/label_facts.py`, which now writes both folders) show pdf.js 6.4.299 and PDFium (pypdfium2 5.14.0) giving the same labels, and qpdf 12.4.2 accepting each file.
- **The web reader** showed a usage-rights file's violation as the approval-signature question ("Page 1 holds a signature"). It now gets the certified file's banner, naming the usage rights (the plan's §6 row).
- **e2e** (`apps/web/e2e/organise.e2e.test.ts`):
  - on the certified P2 file: a move refused with the banner, Apply anyway…, then Save asks to remove the certification; the saved file has the new order, its three bookmarks and no signature;
  - on the approval file: moving the signed page asks nothing, and saves after the signed bytes;
  - on the usage-rights file: turning a page shows the usage-rights banner.
- **The capability matrix.** Every E2 row was at its target in the core and web columns, but no row had any `pdf` evidence: the scanner read TypeScript tags only in the reader's and the desktop's e2e tests (E1f's note). `packages/sdk/test/*.test.ts` is now a suite for the `pdf` column (the fast lane's `WASM` job runs it). The library's 28 claims written as "C.01 (pdf, round-trip): …" are now tags. New tests prove merge (C.06) and split (C.07). All 16 E2 rows are at target in the core, pdf and web columns, and ten other rows gain `pdf` evidence (B.03, H.08, I.06, I.09, I.11, J.01–J.04, J.10).
- **One-page fixtures.** Deleting their only page is refused, and moving or reversing it changes nothing. The suite skips those cases at save time; `would_violate` still judges them, since it can't know an op will change nothing.

## Post-merge checks

None.

## Tests

See tasks; the fixture set is reused by E3–E17 gating tests.

## Docs to update

- `fixtures/README.md` (signed fixtures)
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- pyHanko is MIT; poppler's `pdfsig` is GPL — use it only as an external CLI in CI, never linked (ADR 0019)

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
