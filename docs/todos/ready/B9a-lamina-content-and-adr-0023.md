# B9a — ADR 0023 and `lamina-content`: read-only content-stream interpreter

| Field | Value |
| --- | --- |
| ID | `B9a` |
| Status | `ready` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction), requirement [§4 F8](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (prerequisite) |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B9b](./B9b-text-extraction-core.md) |
| ADRs | Writes **0023** (content-stream reading: `lamina-content` crate, operator and XObject-depth limits); uses [0003](../../decisions/0003-reader-tolerance-and-limits.md) reserved limits `max_content_operators` and `max_xobject_depth`; [0018](../../decisions/0018-test-strategy.md) fuzz target `content` |
| Branch | `feat/b9a-lamina-content` |
| PR title | `feat(content): ADR 0023 and a read-only content-stream interpreter` |
| Size | L (≈ 900–1,300 lines incl. ADR) |
| PR | — |

## Goal

Read what is drawn on a page. A tolerant tokenizer and a read-only interpreter walk a page's content streams (and Form XObjects, recursively and bounded) while tracking graphics and text state, and report typed events (text show, path paint, image, marked-content begin/end, XObject enter/leave) to a visitor. Text extraction ([B9b](./B9b-text-extraction-core.md)) is the first consumer; full-editor E2 (content append) and E7 (editing, ADR 0022) extend the crate later.

## Context

- [Research 03 §2](../../research/pdf/03-text-and-fonts.md#2-text-objects-and-operators-iso-32000-2-9294) (text objects, state, positioning, showing operators, `Tr`, [§2.6 matrices and displacement](../../research/pdf/03-text-and-fonts.md#26-matrices-and-glyph-displacement-math), [§2.7 TJ](../../research/pdf/03-text-and-fonts.md#27-kerning-and-positioning-via-tj)); [§8.1 how extractors work](../../research/pdf/03-text-and-fonts.md#81-how-extractors-work).
- [Research 02 imaging model](../../research/pdf/02-imaging-model.md): graphics state, `q`/`Q`, `cm`, colour operators, XObjects, inline images (`BI … ID … EI`).
- ISO 32000-2 §7.8 (content streams), §8.2–8.4 (graphics state), §9.3–9.4 (text state/objects), §14.6 (marked content), Annex A (operator summary).
- [ADR 0003 §3](../../decisions/0003-reader-tolerance-and-limits.md#3-limits) (`max_content_operators` 10,000,000; `max_xobject_depth` 32; "a fuel counter … may come back with content-stream interpretation (ADR 0023)").
- [pdf-library §5](../../plans/pdf-library.md#5-design-summary) and [full-editor §6](../../plans/full-editor.md#6-design-summary) (E2/E7 extend this crate).
- Existing test-only content code: [`crates/lamina-cos/tests/content.rs`](../../../crates/lamina-cos/tests/content.rs).

## Scope

**In scope**

- ADR 0023: why a separate core crate (`lamina-content`, below `lamina-text`), read-only now and how E2/E7 extend it, visitor vs event-iterator API, tolerance rules (unknown operators skipped with a warning, wrong operand counts/types → operator skipped, unbalanced `q`/`Q` and `BT`/`ET` repaired, inline-image end detection heuristics), limits (operators per stream, XObject depth, cycle detection by object identity, total operators per page = sum), warnings with codes.
- `crates/lamina-content` (core): `Lexer` over decoded content bytes (reusing `lamina-syntax` tokens), operand stack bounded, `Operator` enum covering every Annex A operator (unknown → `Operator::Unknown(name)`), `ContentIter` yielding `(Operator, Operands)`.
- `Interpreter` with `GraphicsState` (CTM, colour spaces/colours, line style, `ExtGState` subset: opacity, font via `/Font`), `TextState` (`Tc`, `Tw`, `Tz`, `TL`, `Tf`, `Tr`, `Ts`, `Tm`, `Tlm`), `q`/`Q` stack (bounded), resources lookup with inheritance (page → Form XObject `/Resources`, fallback to page's when missing — documented tolerance), Form XObject recursion (matrix, bbox clip noted), marked-content stack (`BMC`/`BDC` with property lists from `/Properties`, `EMC`), inline images skipped safely.
- `Visitor` trait with callbacks: `show_text(TextShow { bytes, font: FontRef, state snapshot, ctm, tm })` (glyph decoding is B9b's job), `paint_path`, `image`, `begin_marked(tag, props, mcid)`, `end_marked`, `enter_xobject`/`leave_xobject`, `warning`.
- New warnings/codes: `content-unknown-operator`, `content-bad-operands`, `content-unbalanced-q`, `content-unbalanced-bt`, `xobject-cycle`, `inline-image-unterminated`.
- Fuzz target `content` (interpret arbitrary bytes as a page's content with a fixed resource dict; no panic/hang under default limits).

**Out of scope** (and which todo covers it)

- Font decoding, Unicode mapping, glyph quads: [B9b](./B9b-text-extraction-core.md).
- Bindings and differential tests: [B9c](./B9c-text-extraction-bindings-and-differential.md).
- Writing/editing content streams: full-editor E2, E7.
- Rendering (PDFium/hayro do it): web C2.
- Type 3 font glyph procedures (interpreted later; reported as unsupported now), shading/pattern internals.

## Tasks

- [ ] Load `/rust-best-practices`; read 03 §2 and 02; ISO 32000-2 Annex A
- [ ] Write `docs/decisions/0023-content-stream-reading.md`; index; add to architecture §3 table (row for 0023); move ADR 0003's reserved limits to "used"
- [ ] Create `crates/lamina-content` (core) with `lexer.rs`, `operator.rs`, `state.rs`, `interp.rs`, `visitor.rs`, `resources.rs`
- [ ] `Page::content_iter()` / `interpret(&Page, &mut impl Visitor)` entry on top of `lamina-model`
- [ ] Limits wiring (`max_content_operators`, `max_xobject_depth`, `max_nesting_depth` for the q-stack) with paired tests
- [ ] `fuzz/fuzz_targets/content.rs` + seeds from corpus pages
- [ ] Tests below

## Acceptance criteria

- [ ] Every operator in Annex A parses with correct operand arity; unknown/bad ones warn and continue
- [ ] Interpreting every page of `fixtures/corpus` succeeds or fails with a coded error; operator counts per page match a pdf.js `getOperatorList` count within documented differences (full lane)
- [ ] Nested Form XObjects at depth 32 succeed; depth 33 fails with `LimitExceeded { max_xobject_depth }`; a self-referencing XObject reports `xobject-cycle`
- [ ] CTM and text matrix after each operator match hand-computed values in unit tests (03 §2.6 formulas)
- [ ] Fuzz target `content` runs 10 min without panic (scheduled run linked)
- [ ] ADR 0023 Accepted

## Tests

- Unit tests per module; `crates/lamina-content/tests/{operators,state,xobjects,marked,limits,corpus}.rs`.
- Fuzz `content`; regressions to `fixtures/regressions/`.

## Docs to update

- [ ] [pdf-library §10 B9](../../plans/pdf-library.md#10-to-dos): tick ADR 0023 and "Create `crates/lamina-content`"
- [ ] `docs/decisions/README.md`, [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs) (0023 row), ADR 0003 §3 (reserved → used), `CLAUDE.md` layout

## Risks and open questions

- Visitor vs iterator: a visitor is simpler with XObject recursion; E7 may want an editable op list — ADR 0023 should leave room (e.g. a `ContentOps` representation later).
- Real-world content streams with broken inline images; heuristics from pdf.js (`EI` followed by whitespace and an operator) documented as tolerance rules.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
