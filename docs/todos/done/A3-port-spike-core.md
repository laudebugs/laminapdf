# A3 — Port the spike reader/writer into `lamina-syntax` and `lamina-cos`

| Field | Value |
| --- | --- |
| ID | `A3` |
| Status | `done` |
| Plan | [architecture §5 Round A](../../plans/architecture.md#5-rounds), [§7 Round A to-dos](../../plans/architecture.md#round-a--foundations) |
| Round / milestone | Round A (foundations) — M0 |
| Depends on | [A2](./A2-monorepo-skeleton.md) |
| Blocks | [B1a](./B1a-round-b-adrs.md), [B1b](./B1b-krilla-fit-spike.md) |
| ADRs | Implements [0001](../../decisions/0001-core-language-and-licence.md) (determinism, bytes in/out); its tolerance behaviour was later written down as [0003](../../decisions/0003-reader-tolerance-and-limits.md) |
| Branch | `feat/port-spike-core` |
| PR title | `feat(core): port the spike reader/writer into lamina-syntax and lamina-cos` |
| Size | XL as merged (≈ 6,800 lines incl. fixtures and tests; a straight port, so not split) |
| PR | [#8](https://github.com/laudebugs/lamina/pull/8) (squash-merged as `0f55e72`) |

## Goal

Turn the `pdf-basics` spike's Rust core into the first two product crates, keeping its behaviour and its tests, so Round B builds on reviewed, tested code instead of a spike.

## Context

- [Research ch. 07](../../research/pdf/07-spike-rust-and-typescript.md): what the spike proved and the bugs it found.
- [`spikes/pdf-basics/SPEC.md`](../../../spikes/pdf-basics/SPEC.md): the behaviour that was ported.

## Scope

**In scope**

- [`crates/lamina-syntax`](../../../crates/lamina-syntax/): object model and byte-exact serialisation, lexer, depth-limited parser, Flate + PNG predictor filters (with the bounds fixes), its own `Error`.
- [`crates/lamina-cos`](../../../crates/lamina-cos/): tolerant reader (classic and stream xref, `/Prev`, hybrid `/XRefStm`, object streams, repair, page tree, object cache, `OpenOptions::max_decoded_size`) and the classic/compact/incremental writer (moves to `lamina-save` in [B3b](../ready/B3b-lamina-save-writers.md)).
- `HashMap`/`HashSet` replaced by `BTreeMap`/`BTreeSet`; determinism tests.
- Fixtures moved to `fixtures/` (`golden/`, `corpus/` + `expected.json`) with a provenance and licence `fixtures/README.md`.
- The spike's content builder and golden samples kept as test support in `crates/lamina-cos/tests/common/` until `lamina-build` ([B5a](../ready/B5a-lamina-build-core.md)).
- Review fixes: repair order (last definition in the file wins), generation checks (stale → `null`), `null` page attributes, references inside `/Filter`/`/DecodeParms`, object-stream header lexing, `incremental_update` refusing encrypted bases, matching xref kind and never shrinking `/Size`.

**Out of scope**

- `Limits`, warning codes, owned `Document`: [B2a](../done/B2a-limits-owned-document.md).

## Tasks

- [x] Port and adapt the code to workspace lints (no `unwrap`/indexing in library code)
- [x] Port tests: `golden`, `determinism`, `incremental`, `raw_writer`, `roundtrip`, `policy`, `reader_tolerance`, `reader_cache`, `corpus`, `robustness`, `content`
- [x] Move fixtures and write `fixtures/README.md`
- [x] Fix the review findings listed above

## Acceptance criteria

- [x] Every ported test passes; goldens are byte-identical to the spike's
- [x] `cargo xtask check` and `portability` pass for both crates on all targets

## Tests

The integration tests listed above under `crates/lamina-cos/tests/`, unit tests in both crates.

## Docs to update

- [x] `CLAUDE.md` layout, crate READMEs, architecture §7 Round A ticks

## Risks and open questions

Two known leftovers, both picked up in [B2a](../done/B2a-limits-owned-document.md): the 30 s wall-clock budget in `tests/robustness.rs`, and the borrowed `Document<'a>` with `Rc` caches.

## Definition of done

- [x] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (written retroactively: the todo board did not exist yet)
- [x] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [x] Squash-merged; dependants' "Depends on" now satisfied
