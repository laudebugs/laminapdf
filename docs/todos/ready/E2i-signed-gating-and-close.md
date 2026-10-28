# E2i — DocMDP gating tests on signed fixtures and E2 round close

| Field | Value |
| --- | --- |
| ID | `E2i` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2h](./E2h-page-organiser-ui.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `test/e2i-signed-gating-and-close` |
| PR title | `test(editor): signed-document gating suite for page operations` |
| Size | M (200–600) |
| PR | — |

## Goal

Prove, across all E2 ops, that signed documents are protected exactly as ADR 0007 §2 says: every page op on certified (P1/P2/P3) fixtures is blocked or requires the named-signature override, and the overridden save produces a document whose broken signatures were removed (§3); on approval-only fixtures the same ops save `AppendOnly` as an incremental update that leaves the signature intact, unless they change a field locked by FieldMDP/`/Lock`; then verify the round's capability rows and close the round.

## Context

- [ADR 0007](../../decisions/0007-save-planner.md) §2–§3
- [04 §8.6 Certification vs approval; MDP](../../research/pdf/04-interactive-security.md#86-certification-vs-approval-signatures-mdp)
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) save rules

## Scope

**In scope**

- Extend the `fixtures/signed/` set from E1c with signed documents that have outlines, named destinations, forms and tagged structure (so page ops have something to break)
- Table-driven test over every E2 op × fixture asserting `would_violate`, banner kind and override behaviour
- Validation of overridden outputs: pyHanko/`pdfsig` show no broken signatures, fields remain unsigned
- Capability-matrix check: every E2 row at its target status in every column

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** tests only (`crates/lamina/tests/signed_gating.rs`, e2e spec)
- **`SaveConstraint`:** exercises all kinds
- **Web vs desktop:** e2e checks the banner on both shells
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** verifies (does not newly flip) E2 rows; fixes any row still short of target

## Tasks

- [ ] Extend the fixture generation script and README provenance
- [ ] Table-driven Rust test
- [ ] One e2e per shell for the banner
- [ ] Run `cargo xtask capabilities` and fix gaps
- [ ] Tick E2 to-dos in the plan

## Acceptance criteria

- [ ] Every E2 op × signed fixture case has an asserted verdict: violation under P1, P2 and P3 (`SetPageLabels` and the `PageStructure`/`Content` ops alike); `AppendOnly` with no violation on the approval-only fixture; violation on the FieldMDP-lock fixture only for ops that touch a locked field's widget
- [ ] Approval-only fixture: each E2 op's incremental save validates in pyHanko with the original signature intact (its `/ByteRange` unchanged) and the later revision reported as a modification
- [ ] Overridden saves validate as unsigned (no "broken" signatures) in pyHanko and `pdfsig`
- [ ] All E2 rows at target status

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
