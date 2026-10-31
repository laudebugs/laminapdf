# B2c — `lamina-model` views and IDs, corpus facts, pypdf differential and fuzz targets

| Field | Value |
| --- | --- |
| ID | `B2c` |
| Status | `done` |
| Plan | [pdf-library §7 B2](../../plans/pdf-library.md#b2-document-model), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2a](./B2a-limits-owned-document.md) |
| Blocks | [B3a](./B3a-overlay-and-ops.md), [B9a](./B9a-lamina-content-and-adr-0023.md), [C2b](./C2b-lamina-render-crate.md), [C6a](../ready/C6a-thumbnails-outline-page-labels.md), [X3a](./X3a-nightly-lane.md), [X4](./X4-full-lane-gate.md) |
| ADRs | Implements [0005](../../decisions/0005-document-model.md) §4–§5; [0018](../../decisions/0018-test-strategy.md) tiers 3, 4, 6, 9; follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (per-field tolerance, `max_tree_depth`) |
| Branch | `feat/b2c-lamina-model` |
| PR title | `feat(model): typed page, outline, info and name-tree views with LaminaIds; corpus facts and fuzz targets` |
| Size | L (≈ 1,200–1,500 lines; split the fuzz/differential half into its own PR if review exceeds that) |
| PR | [#23](https://github.com/laudebugs/lamina/pull/23) |

## Goal

Give callers a typed, tolerant read API over the lazy COS graph (pages with their boxes and rotation, the outline, Info, name and number trees, page labels) and the stable `LaminaId`s that ops, undo and sync will address. In the same PR, put the test harness ADR 0018 assigns to B2 in place, so every later round is checked against independent facts and fuzzed from day one: corpus facts from pypdf, a page-count/outline differential, the `cargo xtask golden --update` command and the first `fuzz/` targets with a scheduled 10-minute run.

## Context

- [ADR 0005 §4](../../decisions/0005-document-model.md#4-typed-views-lamina-model): views are thin borrowed wrappers (`Page<'d>`), lazy, never cache copies, per-field tolerance (bad `/MediaBox` → `None` + warning); bindings get owned handles later.
- [ADR 0005 §5](../../decisions/0005-document-model.md#5-laminaids): ID = kind + object number + generation (`p12.0`), path IDs for direct dictionaries (`p5.0/Kids/3`), `/NM` is only a hint, document-level allocator, no reuse.
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers): tier 3 (`cargo xtask golden --update`, "added in B2"), tier 4 (facts from independent tools, exceptions file), tier 6 (differential), tier 9 (fuzz targets `lex`, `parse_object`, `flate_decode`, `open`, `page_tree`; OSS-Fuzz layout; `fixtures/regressions/`).
- [Research 04 §1.2 outline](../../research/pdf/04-interactive-security.md#12-document-outline-1233), [§1.5 page labels](../../research/pdf/04-interactive-security.md#15-page-labels-thumbnails-viewer-preferences-page-modelayout), [§5.1 Info](../../research/pdf/04-interactive-security.md#51-the-info-dictionary-1433); [01 file structure](../../research/pdf/01-file-structure.md) (page tree, inheritance).
- Code: [`lamina-cos/src/reader/mod.rs`](../../../crates/lamina-cos/src/reader/mod.rs) (`Page`, `pages()`), [`tests/common/mod.rs`](../../../crates/lamina-cos/tests/common/mod.rs) (`exercise`, `Rng`), [`fixtures/corpus/expected.json`](../../../fixtures/corpus/expected.json).

## Scope

**In scope**

- New core crate `crates/lamina-model` (`layer = "core"`, `forbid(unsafe_code)`), depending on `lamina-cos`.
- Views: `Pages` (iterator, `len`, `get(index)`, `by_id(PageId)`), `Page<'d>` (`id`, `media_box`, `crop_box`, `bleed/trim/art_box`, `rotate` as `Rotation`, `user_unit`, `resources`, `contents` refs, `annots` refs, `label`), `Outline`/`OutlineItem` (title decoded from PDFDocEncoding/UTF-16BE/UTF-8, children, open/closed count, destination resolved to `PageId` + view when possible, action kind), `Info` (title, author, subject, keywords, creator, producer, creation/mod dates parsed as `PdfDate` with raw fallback), `NameTree`/`NumberTree` (lookup and ordered iteration, `/Kids`/`/Limits` tolerant, `max_tree_depth`), `PageLabels` (styles D/R/r/A/a, prefix, start → label strings).
- Text-string decoding (`PdfDocEncoding`, UTF-16BE with BOM, UTF-8 BOM for PDF 2.0) in `lamina_syntax::text`: the one PDFDocEncoding table in the workspace (code ↔ Unicode, both directions) and `decode_text_string`. This todo comes first, so it owns the table; [B3a](./B3a-overlay-and-ops.md) adds the encoder next to it and [B4c](../done/B4c-standard-14-and-simple-fonts.md) reuses it instead of writing its own.
- `LaminaId` + typed newtypes (`PageId`, `OutlineItemId`; `AnnotId`/`FieldId` declared for later), JSON string form, `FromStr`/`Display`, allocator on the document (interior state), path IDs for direct dictionaries.
- New warnings with codes (in `fixtures/codes.json`): `invalid-box`, `invalid-rotate`, `name-tree-node-skipped`, `outline-cycle`, `invalid-date`, `invalid-page-label`.
- Corpus: `fixtures/corpus/expected.json` gains `outline_titles` (flattened, depth-first) and keeps `page_count`, produced by a script `tools/corpus-facts/` (Python + pypdf, pinned) — **never by lamina**; the script reads and writes only `fixtures/corpus/` (encrypted fixtures and their facts live in `fixtures/encrypted/` + `fixtures/encrypted/expected.json`, owned by [B2b](../done/B2b-standard-security-decryption.md)); `fixtures/corpus/exceptions.toml` listing deliberate differences with the ADR 0003 rule.
- Corpus test `crates/lamina-model/tests/corpus.rs`: every file opens or fails with a coded error; facts match.
- `cargo xtask golden --update` regenerating every golden under `fixtures/golden/` from its source and printing the list.
- `cargo xtask differential` (pypdf only in this PR): runs pypdf over `fixtures/corpus` + a fixed-seed mutant set, compares page count, outcome (opened/repaired/failed) and outline titles; `--update` writes precomputed expectations to `fixtures/differential/expected.json`.
- `fuzz/` (outside the workspace, nightly toolchain, OSS-Fuzz layout): `Cargo.toml`, targets `lex`, `parse_object`, `flate_decode`, `open` (open and drive every read path like `tests/common::exercise`, plus the model views), `page_tree`; seed corpus script from `fixtures/`; `cargo xtask fuzz <target> [--time N]`.
- `.github/workflows/fuzz.yml`: scheduled (nightly) and `workflow_dispatch`, 10 min per target, uploads crashes as artefacts; [X3a](../done/X3a-nightly-lane.md) later folds it into the nightly lane.

**Out of scope** (and which todo covers it)

- Annotations, fields, structure tree views: full-editor E-rounds.
- Mutation, overlay, op application: [B3a](./B3a-overlay-and-ops.md).
- pdf.js, PDFium and qpdf in the differential, the 500-mutant set in the PR lane: [X4](./X4-full-lane-gate.md) and [X3a](../done/X3a-nightly-lane.md).
- Fuzzing in the PR lane (2 min per target): [X4](./X4-full-lane-gate.md).

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0005 §4–5, ADR 0018 §1
- [x] Create `crates/lamina-model` (Cargo.toml with `[package.metadata.lamina] layer = "core"`, README, `lib.rs` docs)
- [x] `id.rs`: `LaminaId`, `Kind`, newtypes, string form, parser, allocator; unit tests for round-trip and uniqueness
- [x] `crates/lamina-syntax/src/text.rs`: the PDFDocEncoding table (code → Unicode and the reverse lookup, ISO 32000-2 Annex D) and `decode_text_string`, with tests (every table entry, UTF-16BE surrogates, invalid sequences → U+FFFD); `lamina-model` uses it
- [x] `page.rs`: `Pages`, `Page<'d>`, box rectangles normalised (`Rect` with min/max), inheritance via the COS `Page`, per-field warnings
- [x] `outline.rs`: iterative walk with a visited set (cycles → `outline-cycle`, stop), `max_tree_depth`, destination resolution (explicit arrays, named dests via `/Dests` and the `/Names` tree)
- [x] `info.rs`, `date.rs` (PDF date parser with tolerant fallbacks)
- [x] `tree.rs`: generic name/number tree with lookup and iteration
- [x] `labels.rs`: page labels
- [x] Re-export `Limits`, `OpenOptions`, `Warning`, `Document` from `lamina-model` (façade order per ADR 0005 §1)
- [x] `tools/corpus-facts/`: `requirements.txt` (pinned pypdf), `facts.py` writing `expected.json`; document in `fixtures/README.md`
- [x] `crates/lamina-model/tests/corpus.rs`, `ids.rs` (stable across reopen and after an incremental no-op save with `lamina-cos`'s `incremental_update`), `views.rs`
- [x] `tools/xtask`: `golden --update`, `differential [--update]`, `fuzz <target> [--time]`, `corpus` helpers; wire `differential` (precomputed expectations, no Python needed) into `cargo xtask test`
- [x] `fuzz/`: five targets, `fuzz/README.md` (how to run, how to minimise into `fixtures/regressions/`), seeds script
- [x] Fuzz seeds for encrypted files are owned by [B2b](../done/B2b-standard-security-decryption.md), which writes them to `fuzz/seeds/open/encrypted/` and their passwords to `fuzz/seeds/open/passwords.txt` (creating the directory if this PR hasn't merged yet). Here: the seeds script includes everything under `fuzz/seeds/<target>/`, and the `open` target tries the empty password and then each line of `fuzz/seeds/open/passwords.txt` (`include_str!`). If that file isn't on `main` yet, create it empty (a comment line) for B2b to fill; this works in either merge order
- [x] `.github/workflows/fuzz.yml` (cron, nightly toolchain, `cargo fuzz run <t> -- -max_total_time=600 -timeout=10 -rss_limit_mb=2048`)
- [x] Add `fuzz/` exclusion check to xtask if not already present

## Acceptance criteria

- [x] Every file in `fixtures/corpus` opens or fails with a coded error; page count and outline titles equal pypdf's facts in `expected.json` except entries in `exceptions.toml`, each with its ADR 0003 rule
- [x] `LaminaId`s for every page and outline item are identical after reopen and after an incremental save; a moved page (built by test code) keeps its `PageId`
- [x] A direct page dictionary gets a path ID that is stable for the same bytes
- [x] `cargo xtask golden --update` on a clean tree produces no diff
- [x] `cargo xtask differential` passes in the fast lane using precomputed expectations; `--update` regenerates them with pypdf
- [x] Each fuzz target runs 10 minutes locally (`cargo xtask fuzz <target> --time 600`) without a panic, hang or OOM under default `Limits`; summary in the PR
- [ ] `.github/workflows/fuzz.yml` passes `actionlint`; its job runs once on the branch (a temporary `pull_request` trigger with `-max_total_time=60`, removed before merge, or `act`), run linked in the PR
- [x] Every model view hitting `max_tree_depth` degrades with a warning (paired limit tests)

## Notes (as built)

- `lamina-model`'s `Document` wraps `lamina_cos::Document` (and dereferences to it) so the ID allocator can live on the document as interior state; `Limits`, `OpenOptions`, `Warning`, `Error` and `Result` are re-exported from `lamina-cos`. Views raise their warnings on the COS document through the new `lamina_cos::Document::report`.
- Two codes beyond the six listed: `invalid-entry` (an entry a view reads, such as `/Info` or `/UserUnit`, of the wrong type) and `outline-item-skipped` (an outline item that is missing, not a dictionary or deeper than `max_tree_depth`), the outline's equivalent of `page-tree-node-skipped`; `name-tree-node-skipped` covers number trees (page labels) too. All eight are in `fixtures/codes.json` and ADR 0003 §2.
- For path IDs, the COS page-tree walk records a `DirectPath` (nearest indirect ancestor plus keys and indices) for direct page dictionaries, and exposes the stored inheritable attributes (`Page::inherited`) so views can tell an absent box from a malformed one.
- `fixtures/corpus/outline.pdf` (generator: `tools/fixtures/outline.py`) is new: the corpus had no outlines, page labels or named destinations to compare.
- Goldens are regenerated by `crates/lamina-cos/examples/golden.rs`; the differential's inputs by `crates/lamina-model/examples/differential-inputs.rs`, which shares `tests/common/mutants.rs` with the test. `cargo xtask differential` runs that test, which `cargo xtask test` also runs.
- `fuzz.yml` ran once on the branch through a temporary `pull_request` trigger (60 s per target), green on every target: [run 40810183790](https://github.com/laudebugs/lamina/actions/runs/40810183790); the trigger was reverted before merge. The first run failed because the prebuilt cargo-fuzz defaults to the musl target; `cargo xtask fuzz` now passes the nightly host triple. `actionlint` was not run locally (downloading its binary was not permitted in this environment); GitHub accepted and ran the workflow, so that criterion stays open for a reviewer with actionlint.
- Fuzz crash inputs are kept per target as raw bytes in `fixtures/regressions/<target>/` and replayed by `crates/lamina-model/tests/regressions.rs` through `tests/common/fuzz_entry.rs`, the same functions the fuzz targets call (not as `<target>-<hash>.pdf` files with one hand-written test each).
- The PR was not split into B2d (that ID is now windowed reading); the fuzz and differential half is reviewable on its own (`fuzz/`, `tools/`, `fixtures/differential/`).

## Post-merge checks

- [ ] The first scheduled `fuzz.yml` run on `main` is green for every target (10 min each); link it in a follow-up PR or the done todo

## Tests

- Unit tests in each `lamina-model` module.
- Integration: `crates/lamina-model/tests/{corpus,ids,views,limits}.rs`.
- Corpus facts: `fixtures/corpus/expected.json` (pypdf), `exceptions.toml`.
- Differential: `fixtures/differential/expected.json` (pypdf over corpus + fixed-seed mutants, seed printed).
- Fuzz: `fuzz/fuzz_targets/{lex,parse_object,flate_decode,open,page_tree}.rs`; crashes → `fixtures/regressions/<target>-<hash>.pdf` + named test.

## Docs to update

- [x] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos): tick `lamina-model`, `LaminaId`s, `expected.json` + corpus test, `fuzz/`, differential vs pypdf
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "`lamina-model`" (pages/outline part; note annotations/fields later) and the fuzz half of the harness to-do
- [x] `CLAUDE.md` (new crate in layout; `golden`, `differential`, `fuzz` xtask commands; `fuzz/` folder), `fixtures/README.md`, crate README

## Risks and open questions

- pypdf disagrees with ADR 0003 on some damaged files: record each in `exceptions.toml` rather than bending the reader.
- Outline destinations can point to pages by object or by index (remote/broken); define `OutlineItem::target()` as `Option<Destination>` and keep unresolved raw data.
- Fuzzing needs nightly Rust; keep `fuzz/` out of the workspace (as ADR 0018 says) and out of `rust-toolchain.toml`.
- If the PR grows past ~1,500 lines, split off `B2d` (xtask differential + fuzz) and chain it; update this file and the plan links.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
