# L3f — The extraction benchmark: reading order and tables on a licence-clean public subset

| Field | Value |
| --- | --- |
| ID | `L3f` |
| Status | `done` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction) ("Accepted when": scores measured and recorded) |
| Round / milestone | Round L3 (shareable libraries) — AI-ready extraction |
| Depends on | [L3e](./L3e-extract-bindings-and-benchmark.md) |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md), [0018](../../decisions/0018-test-strategy.md) (not in the PR lanes), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (data and metric licences) |
| Branch | `feat/l3f-extract-bench` |
| PR title | `feat(xtask): extract-bench, reading-order and table scores on a public subset` |
| Size | M (≈ 400–700 lines: manifest, fetch, metrics, report) |
| PR | [#253](https://github.com/laudebugs/laminapdf/pull/253) |

## Goal

Measure what the rule-based extractor achieves, so users know before reaching for models (L3c). Run `laminapdf-extract` over a pinned public subset of born-digital PDFs with ground truth, score its reading order and its tables, and record the scores with the dataset, licence, subset, version and date.

> **Outcome (2026-11-05).** olmOCR-bench (ODC-BY-1.0) is the dataset: its three born-digital subsets score 52.8 % (reading order), 24.1 % (tables) and 37.6 % (headers and footers left out) with the default tables: 50.1 % as the benchmark averages them with its baseline group, 38.2 % without it. The scores are the benchmark's own tests, run from its pinned package, rather than TEDS: no permissively licensed dataset with PDFs and table structure was reachable (FinTabNet's host refuses this environment). The manifest records every dataset and subset left out, with the reason. Two findings: 13 pages use filters `laminapdf-syntax` doesn't decode yet ([#108](https://github.com/laudebugs/laminapdf/issues/108)), and one undecodable page fails the whole document's extraction.

## Context

- Split from [L3e](./L3e-extract-bindings-and-benchmark.md) at its size limit (2026-11-05); [L3b's todo](./L3b-extract-tables-and-serialisers.md) has the design (its Benchmark bullet) and the licence risks.
- [AI extraction note §B](../../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based): the benchmarks (olmOCR-Bench, OmniDocBench, OpenDataLoader's) and their metrics.

## Scope

**In scope**

- `tools/bench/extract-datasets.toml`: each dataset's URL, SHA-256 per file, licence (with the link to it), the subset's IDs and why they were chosen; only PDF inputs, never page images.
- `cargo xtask extract-bench [--dataset <name>] [--sample N]`: fetches into `target/bench/` (never committed unless the licence allows), runs the extraction, and scores reading order (normalised edit distance over block order, as OmniDocBench defines it, or the dataset's own metric) and tables (TEDS, implemented in xtask or from a reference under a permissive licence); writes `target/bench/extract/report.json` and a Markdown table.
- The scores in `crates/laminapdf-extract/README.md`, with dataset, licence, subset size, Lamina PDF version and date, and a note on comparing with vendor scores.

**Out of scope**

- Model plug-ins → [L3c](../ready/L3c-model-plugins.md); structure-tree order → [L3d](./L3d-extract-structure-and-furniture.md).

## Tasks

- [x] Check each candidate dataset's licence (OmniDocBench, olmOCR-Bench, OpenDataLoader's, PubTables-1M, FinTabNet, DocLayNet); record the decisions in the manifest
- [x] The manifest and the fetch with SHA-256 checks; the metrics are the benchmark's own tests (olmOCR-bench's pinned package), so there is no metric code of ours to unit-test
- [x] Run it; record the scores
- [x] Not in the nightly lane: it fetches about 750 files and needs Python with the scorer, so it runs by hand (`CLAUDE.md`, `tools/bench/README.md`)

## Acceptance criteria

- [x] `cargo xtask extract-bench` runs on the pinned subset and writes the report
- [x] Scores recorded with dataset, licence, subset size, version and date
- [x] No dataset file is committed unless its licence allows redistribution

## Post-merge checks

- [x] Not applicable: `extract-bench` didn't join the nightly lane

## Tests

`extract_bench`'s unit tests (arguments, sampling, the lock, the manifest); the metrics are the benchmark's own tests, and the bench itself runs outside the PR lanes.

## Docs to update

- [x] [shareable-libraries §9 L3](../../plans/shareable-libraries.md#9-to-dos): tick the benchmark
- [x] `crates/laminapdf-extract/README.md` (scores), `tools/bench/README.md` (the manifest), `CLAUDE.md` (the command)

## Risks and open questions

- **Licences and formats:** some benchmarks' inputs are page images or carry non-commercial licences; use only born-digital PDFs under licences that allow this use, and say which were refused and why.
- **Comparisons:** vendor scores (OpenDataLoader) use other data or metrics; state that next to ours.

## Definition of done

- [x] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
