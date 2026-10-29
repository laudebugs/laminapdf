# B1a — Round B ADRs 0003, 0005, 0006, 0007, 0018

| Field | Value |
| --- | --- |
| ID | `B1a` |
| Status | `done` |
| Plan | [pdf-library §7 B1](../../plans/pdf-library.md#b1-decisions-and-spike), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [A3](./A3-port-spike-core.md) |
| Blocks | [B2a](./B2a-limits-owned-document.md) |
| ADRs | Writes [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0018](../../decisions/0018-test-strategy.md) |
| Branch | `docs/round-b1-adrs` |
| PR title | `docs(decisions): ADRs 0003, 0005, 0006, 0007, 0018 (Round B1)` |
| Size | M (≈ 520 lines of docs) |
| PR | [#10](https://github.com/laudebugs/lamina/pull/10) (squash-merged as `3414b9a`) |

## Goal

Write down, before any Round B code, how the reader tolerates damage and bounds work, how the document model owns input and tracks edits, how ops/undo work, how saving stays safe for signatures and redactions, and how all of it is tested.

## Context

- The ported reader ([A3](./A3-port-spike-core.md)): 0003 documents what it already does.
- [Report: editing](../../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving) and [editing notes](../../research/architecture/notes/editing-architecture.md).

## Scope

**In scope**

- 0003: tolerance rules table, stable kebab-case warning codes, strict mode, `Limits` (hard/soft, defaults), enforcement tests.
- 0005: `ByteSource` and owned `Document`, base + overlay + rebase states, typed views, `LaminaId` rules, `Send + !Sync`; §6 generation strategy left Proposed for the spike.
- 0006: `Op` enum (adjacently tagged serde), transactions and savepoints, undo computed at undo time, barrier ops, versioning, dirty state.
- 0007: `SavePlanner`, `SaveConstraint` (signatures, DocMDP, rewrite reasons, violations), mode resolution, canonical order, writers, `WriteSink`, determinism, writer move.
- 0018: test tiers, CI lanes (fast/full/nightly/release), corpora, fuzzing, no wall-clock assertions in PR lanes, flaky policy.

**Out of scope**

- The generation-strategy decision: [B1b](./B1b-krilla-fit-spike.md). ADRs 0016/0017: [B6a](../ready/B6a-lamina-wasm-and-adr-0016.md), [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md).

## Tasks

- [x] Write the five ADRs and index them
- [x] Update architecture §3/§7, pdf-library B1 to-dos, `CLAUDE.md`
- [x] Address review findings

## Acceptance criteria

- [x] All five ADRs Accepted (0005 §6 Proposed until B1b merged)

## Tests

None (documentation only).

## Docs to update

- [x] `docs/decisions/README.md`, `docs/plans/architecture.md`, `docs/plans/pdf-library.md`, `CLAUDE.md`

## Risks and open questions

`Limits` defaults are informed guesses; the nightly corpus run ([X3a](../ongoing/X3a-nightly-lane.md)) will show if one rejects legitimate files.

## Definition of done

- [x] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (written retroactively: the todo board did not exist yet)
- [x] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [x] Squash-merged; dependants' "Depends on" now satisfied
