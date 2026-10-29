# 0023 — Content-stream reading: the `lamina-content` crate, its tolerance rules and limits

- Status: Accepted
- Date: 2026-10-29

## Context

Text extraction ([library plan B9](../plans/pdf-library.md#b9-text-extraction)), the web reader's text layer, search, and later the editor's append (E2), flattening (E5) and content editing (E7, [full-editor plan §6](../plans/full-editor.md#6-design-summary)) all need to know what a page draws: which operators run, with which graphics and text state, inside which Form XObjects and marked-content sequences. A content stream is a postfix program of operands and operators over the PDF token grammar ([ISO 32000-2 §7.8.2](../research/pdf/02-imaging-model.md#2-content-streams)); the 73 operators are listed in Annex A (Table A.1). Interpreting one is the reader's largest attack surface after the file structure: every operand count, nesting level and XObject reference is attacker-controlled, and real files break the rules routinely: unbalanced `q`/`Q`, text operators outside `BT`/`ET`, unknown operators, missing resources, inline images without a length ([research 02 §14](../research/pdf/02-imaging-model.md#14-implications-for-laminas-api)).

[ADR 0003](0003-reader-tolerance-and-limits.md) reserved two limits for this work, `max_content_operators` (10,000,000) and `max_xobject_depth` (32), and deferred a fuel counter to it. Until now the only content code was test support that *writes* content (`crates/lamina-cos/tests/common/content.rs`).

## Decision

### 1. A core crate below `lamina-text`

`crates/lamina-content` (layer `core`: bytes in, bytes out, `#![forbid(unsafe_code)]`) depends on `lamina-syntax` (tokens, objects, `Limits`), `lamina-cos` (resolving resources, decoding Form XObjects, warnings) and `lamina-model` (its `Page`). It sits below `lamina-text` (B9b, glyph decoding and Unicode) and is extended later, not replaced: E2 adds an append API that never parses existing operators (it uses this crate's tokenizer only to balance `q`/`Q`), and E7 (ADR 0022) adds an editable representation. The crate is **read-only** now.

### 2. API: a tokenizer and a visitor

- **`ContentIter`** tokenizes decoded content bytes with `lamina-syntax`'s `Lexer` (array and dictionary operands through its `Parser`) and yields `Lexed::Op(Op)` (an `Operator`, the operands read since the previous one, the byte offset, and for `BI` the whole inline image) or `Lexed::Issue` for what it skipped. `Operator` has one variant per Annex A operator plus `Unknown(&[u8])`; `Operator::signature()` gives each one's operands (Table A.1 and the tables it cites), and `Signature::check` picks the operands an operator uses.
- **`interpret_page(doc, page, visitor)`** and **`interpret_content(doc, bytes, resources, visitor)`** run content against a `GraphicsState` (CTM, line style, colours and colour spaces, rendering intent, flatness, an `ExtGState` subset: `LW LC LJ ML D RI FL CA ca Font`) with its `TextState` (`Tc Tw Tz TL Tf Tr Ts`), the text and text-line matrices, the `q`/`Q` stack, resources, Form XObjects and marked content, and report to a **`Visitor`**: `operator` (every operator as read), `show_text`, `path_segment`, `paint_path`, `paint_shading`, `image` (XObject or inline), `begin_marked`/`end_marked`/`marked_point` (tag, property list inline or from `/Properties`, MCID), `enter_xobject`/`leave_xobject` (matrix, `/BBox`, depth) and `warning`. All methods have empty defaults. They return a `Summary` (operators read, forms drawn).
- **Glyph positions stay with the font.** The interpreter can't advance the text matrix without glyph widths, which are B9b's. So `show_text` returns a `TextAdvance` (the §9.4.4 displacement summed over the shown glyphs, which `TextState::glyph_advance` computes per glyph), and the interpreter applies `Tm = [1 0 0 1 tx ty] × Tm`. The default returns no movement.
- **Why a visitor, not an event iterator.** Form XObjects recurse: an iterator would have to hand out events borrowed from a stack of decoded forms and resource scopes, or copy them. A visitor keeps that stack on the interpreter's call stack and lends each event for the duration of one call. `ContentIter` stays available for callers that only need the flat operator list of one stream (E2's `q`/`Q` balancing, tests). E7's editable list (a `ContentOps` representation with object identity, ADR 0022) is built from `ContentIter` and does not change the visitor.

### 3. Tolerance rules

Each rule has a named test in `crates/lamina-content/tests/`. Every repair raises a warning (§5); strict mode turns it into `Error::Strict`.

| Situation | Behaviour | Warning |
| --- | --- | --- |
| Unknown operator | Skipped with its operands; silently inside `BX … EX` (§7.8.2) | `content-unknown-operator` |
| Too few operands, or one of the wrong type | The operator is skipped | `content-bad-operands` |
| Extra operands | The last ones are used, as a PostScript interpreter pops them; the rest are dropped | `content-bad-operands` |
| More than 64 operands before one operator | The oldest are dropped (no operator takes more than 33: `scn` with 32 DeviceN components and a name, Annex C) | (with the operator's own check) |
| Bytes that are not a token: unbalanced `)`, `>`, `]`, `>>`, a bad hex digit, an unterminated string, array or dictionary; operands with no operator at the end; a stray `ID` or `EI` | Skipped together with the operands read so far; reading resumes after the bad byte | `content-syntax` |
| `Q` with no saved state | Ignored | `content-unbalanced-q` |
| A stream ending with saved states | They are restored; each content stream balances on its own (a page's `/Contents` array is one stream, §7.8.2; each form is one) | `content-unbalanced-q` |
| `BT` inside a text object, `ET` outside one, a text positioning or showing operator outside one, a stream ending inside one | `BT` resets the matrices, a stray `ET` is ignored, a text operator runs with the current matrices, the object is closed at the end | `content-unbalanced-bt` |
| `q`/`Q`/`cm` inside `BT … ET`, path operators in the wrong place | Run as written (renderers do; they are not repairs) | — |
| `EMC` without a sequence; a stream ending with sequences open | Ignored; the open ones are closed (`end_marked` is called for each) | `content-unbalanced-marked` |
| A named resource (font, XObject, `ExtGState`, colour space, pattern, shading, property list) missing, of the wrong type or unreadable | `Tf` keeps the name with no font dictionary, `cs` keeps the name, `BDC` begins the sequence without properties; `Do`, `sh`, `gs` are skipped | `content-resource-missing` |
| A Form XObject without `/Resources` | Uses its parent's resources (the page's for a form drawn by the page), as PDF 1.x allowed (§7.8.3) | — |
| A Form XObject whose content can't be decoded (corrupt data, unsupported filter, a decode limit) | Skipped, like a missing resource (ADR 0003 §3: a hard limit inside a tolerant path degrades with its warning) | `content-resource-missing` |
| A Form XObject that draws itself, directly or through others (detected by object identity on the stack of forms being drawn) | The inner `Do` is skipped; drawing the same form twice in sequence is not a cycle | `xobject-cycle` |
| Inline image end (§8.9.7) | In order: the declared `/L` (`/Length`, PDF 2.0); for an unfiltered image the length computed from `/W /H /BPC /CS` (or `/IM`); then the pdf.js heuristic: the first `EI` preceded by white-space and followed by white-space, a delimiter or the end, whose next 64 bytes are printable ASCII or white-space. A length is used only if `EI` follows it | — |
| No `EI` found | The rest of the stream is skipped | `inline-image-unterminated` |
| Type 3 glyph procedures, patterns' and shadings' internals, clipping, optional content visibility | Not interpreted now: `d0`/`d1` are read and ignored; the visitor gets the dictionaries (`/BBox` is reported with the form, not applied) | — |

Only strict-mode refusals, unavailable input (`DataPending`, `Source`) and the hard limits of §4 end an interpretation; errors decoding the page's own content come from `Document::page_content` and end it too.

### 4. Limits

The limits are those of ADR 0003 §3, now enforced (that table moves them from "reserved" to "used"). All are hard: they end the call with `Error::LimitExceeded`.

| Limit | What it bounds here |
| --- | --- |
| `max_content_operators` (10,000,000) | Operators read by one call, **all streams together**: the page's content plus every Form XObject each time it is drawn. This is the fuel counter ADR 0003 deferred: per-stream counts would not stop forms that draw each other twice per level (2^32 operators from a few hundred bytes). |
| `max_xobject_depth` (32) | Form XObjects being drawn at once: 32 nested forms succeed, the 33rd fails. Patterns and Type 3 glyphs will count against it when they are interpreted. |
| `max_nesting_depth` (256) | Saved graphics states (`q`) per content stream, so a run of `q` can't grow memory without bound (arrays and dictionaries among the operands are bounded by the parser, as before). |
| `max_decoded_total` (1 GiB) | Bytes interpreted by one call, forms counted each time they are drawn, so a large form drawn many times can't keep the tokenizer busy without consuming fuel. Each indirect form is decoded once per call and cached. |
| `max_string_length` (16 MiB) | String and name operands (the lexer's existing limit). |

Marked-content nesting needs no limit: the interpreter keeps only a count. Path segments are streamed to the visitor, not collected.

### 5. Warnings

Nine codes join ADR 0003 §2's table, as `lamina_cos::Warning` variants (so they are recorded on the document like every other warning, deduplicated by code, object and offset, and capped by `max_warnings`), and are listed in `fixtures/codes.json`: `content-unknown-operator`, `content-bad-operands`, `content-unbalanced-q`, `content-unbalanced-bt`, `content-unbalanced-marked`, `content-syntax`, `content-resource-missing`, `inline-image-unterminated` and `xobject-cycle`. Each carries the content's object (the page for page content, the form for a form; `None` for `interpret_content`) and the byte offset in the decoded content (`xobject-cycle`: the form reached again). The visitor sees each warning before it is recorded.

### 6. Tests

- Unit tests per module; `crates/lamina-content/tests/{operators,state,xobjects,marked,limits,corpus,regressions}.rs`: every Annex A operator read with its signature and run without an operand warning, every operator with one operand missing, CTM and text matrices after each operator against values computed by hand from §8.3.4, §9.4.2 and §9.4.4, the limits in pairs (at the limit succeeds, one past fails), each tolerance rule above.
- **pdf.js comparison** (ADR 0018 tier 4): `tools/corpus-facts/pdfjs-operators.mjs` (pdfjs-dist, pinned) writes pdf.js's `getOperatorList` counts for every page of `fixtures/corpus/`, `fixtures/golden/` and `fixtures/content/operators.pdf` (a page using every construct, `tools/fixtures/content.py`) to `fixtures/corpus/pdfjs-operators.json`; `tests/corpus.rs` maps lamina's operators to pdf.js's names and compares the counts exactly. The documented differences, all in pdf.js's representation rather than in what is drawn: path construction is folded into the `constructPath` of the painting operator; `CS`/`cs`, `MP`/`DP` and `BX`/`EX` produce no pdf.js operator; colours become `set*RGBColor` (or `set*ColorN` for a pattern); `'` and `"` expand to their component operators; `dependency` operators are not content and are left out. Files lamina can't interpret yet are listed with their error code and ADR 0003 rule in `fixtures/corpus/pdfjs-exceptions.toml`. The test runs in every lane (it is part of `cargo xtask check`, so the full lane runs it on Linux, macOS and Windows); pdf.js itself runs only when the facts are regenerated.
- Fuzz target `content` (ADR 0018 tier 9): whole files page by page, or arbitrary bytes as a page's content with a fixed resource dictionary holding every resource category, a form that inherits the page's resources and one that draws itself; invariants: no panic, no hang, balanced marked-content and form events.

## Consequences

- B9b builds text extraction on `show_text` and the `TextState`; it computes glyph advances and returns them, and gets correct text matrices across `Tj` sequences without the interpreter knowing about fonts.
- The page's warnings now include content problems; a long broken content stream can fill `max_warnings` (then `warnings-truncated`), which hides later warnings of the same document. Raising the cap is the remedy if that proves noisy.
- The q-stack and operator limits are hard: a page past them fails instead of rendering partially. The defaults are far above what the corpus uses; a legitimate file that hits one raises the default in a normal PR (ADR 0003's consequence).
- Form decoding is cached per call, so memory for one interpretation can reach the decoded size of all forms a page draws (bounded by `max_decoded_total`).
- `lamina-content` depends on `lamina-model` only for `Page`; `interpret_content` takes a COS document and bytes, so appearance streams (E4/E5) can use it without a page.

## Alternatives considered

- **Interpretation inside `lamina-text`.** Text is the first consumer, not the only one: the editor's append and redaction, and render-free inspection, need the same walk without fonts.
- **An event iterator.** Simpler to consume in a loop, but recursion into forms either borrows from a stack the iterator owns (self-referential) or copies every event; a visitor keeps borrowing simple. The flat `ContentIter` covers the iterator use.
- **Separate `ContentWarning` type reported only to the visitor.** It would bypass strict mode and the document's warning list (ADR 0003 §2 says every repair is a `Warning` on the document).
- **A per-stream operator limit.** Doesn't bound work when forms are drawn repeatedly; the per-call count does both.
- **Soft q-stack limit (drop the `q`).** Would silently pair the following `Q` with the wrong state; failing is safer and the default (256) is rarely approached.
- **Collecting path segments into `paint_path`.** Memory then grows with the path; streaming them lets each visitor decide.
- **Comparing raw operator counts with pdf.js.** pdf.js's list is not the content's operator list (it folds paths and colour spaces); mapping lamina's operators to pdf.js's representation keeps the comparison exact and the differences written down.
