# B1b — `krilla-fit` spike: can krilla generate into lamina's object graph?

| Field | Value |
| --- | --- |
| ID | `B1b` |
| Status | `done` |
| Plan | [pdf-library §7 B1](../../plans/pdf-library.md#b1-decisions-and-spike), [architecture §4](../../plans/architecture.md#4-spikes-to-close-open-questions) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [A3](./A3-port-spike-core.md) |
| Blocks | [B4a](./B4a-font-loading-and-subsetting.md), [B5a](./B5a-lamina-build-core.md) |
| ADRs | Settles [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (Option B) |
| Branch | `spike/krilla-fit` |
| PR title | `spike(krilla-fit): can krilla generate into lamina's object graph? (Round B1)` |
| Size | L (≈ 3,000 lines incl. two lockfiles; throwaway spike code) |
| PR | [#12](https://github.com/laudebugs/lamina/pull/12) (squash-merged as `72fe81f`) |

## Goal

Decide the generation strategy with measurements: depend on krilla, or build `lamina-build` over `lamina-cos` reusing `pdf-writer`, `subsetter`, `skrifa` and HarfRust.

## Context

- [Spike README](../../../spikes/krilla-fit/README.md): question, method, measurements, recommendation.
- [Research ch. 05 §8](../../research/pdf/05-creating-pdfs-from-code.md#8-implications-for-lamina).

## Scope

**In scope**

- `spikes/krilla-fit/` (outside both workspaces): experiment A (krilla as-is, opened and incrementally edited by lamina-cos), B1 (copy krilla pages into an existing graph, with renumbering and catalog-conflict reporting; tagging dropped), B2 (pdf-writer chunk in lamina's numbering), C (≈ 400-line lamina-native builder with subset Type 0 font, `/W`, ToUnicode, PNG + SMask, JPEG), WASM size probe.
- Licence check of krilla's tree against `deny.toml`.

**Out of scope**

- Product code: [B4a](../done/B4a-font-loading-and-subsetting.md)–[B5c](../ongoing/B5c-lamina-facade-and-examples.md).

## Tasks

- [x] Build experiments A, B1, B2, C and the WASM probe
- [x] Measure time, size, determinism, open/edit cost; check licences
- [x] Write the README answer and recommendation; update ADR 0005 §6, plans and `CLAUDE.md`
- [x] Review fixes (B1 `/Parent`, inherited attributes, (num, gen) keys, dangling refs, never import tagging; re-measure)

## Acceptance criteria

- [x] README answers the question with measurements (it does: krilla only emits finished bytes; recommend Option B)

## Tests

Spike only: every output passes `pypdf.PdfReader(strict=True)`; A and C pages rasterise identically.

## Docs to update

- [x] Spike README, ADR 0005 §6, architecture §4/§7, pdf-library §5/§9/§10, `CLAUDE.md`

## Risks and open questions

**Carried into B6:** WASM gzip is 377 KB for the lamina builder stack *before* a shaper, over the 350 KB budget. B6a must split base/text modules, re-measure with wasm-opt and without `anyhow`/`png`, and revise the budget in ADR 0016 if needed.

## Definition of done

- [x] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (written retroactively: the todo board did not exist yet)
- [x] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [x] Squash-merged; dependants' "Depends on" now satisfied
