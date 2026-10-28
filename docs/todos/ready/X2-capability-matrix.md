# X2 — Capability matrix generator: ~180 operations, test-backed statuses

| Field | Value |
| --- | --- |
| ID | `X2` |
| Status | `ready` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting), [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) |
| Round / milestone | Cross-cutting (before the editor rounds; useful from M1) |
| Depends on | [B3c](./B3c-save-planner-and-roundtrip.md) |
| Blocks | [E1a](./E1a-editing-foundation-feature-plan.md), [E1f](./E1f-metadata-ui-capability-page.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) (tests are the evidence), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (xtask) |
| Branch | `build/x2-capability-matrix` |
| PR title | `build(xtask): test-backed capability matrix for the ~180 operations` |
| Size | L (≈ 700–1,100 lines incl. the manifest) |
| PR | — |

## Goal

Make progress measurable and honest: a manifest of every operation in the [feature inventory](../../research/pdf/06-feature-inventory.md) (~180 rows, stable IDs `<group>.<nn>`), test attributes that tie tests to a row and a status, and `cargo xtask capabilities` that runs the tagged tests and regenerates `docs/capabilities/README.md`. CI fails if a row claims a status no passing test proves.

## Context

- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix): rows, columns (`core`, `pdf`, `web`, `desktop`, `target round`), statuses (`unsupported`, `read`, `write`, `round-trip`), mechanics (manifest TOML, test attributes, generated README), "done means".
- [Research 06 §3](../../research/pdf/06-feature-inventory.md) (the operation list), [§10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina).
- [Roadmap: how we track progress](../../plans/roadmap.md#how-we-track-progress).

## Scope

**In scope**

- `docs/capabilities/manifest.toml`: one entry per operation in 06 §3 (ID, name, group, target round per surface), generated once by a script from 06 and then maintained by hand.
- Rust test tagging: a tiny proc-macro-free convention, e.g. `#[test] #[doc = "capability: C.04 core round-trip"]`-style marker or a `capability!("C.04", Core, RoundTrip)` macro registering via `inventory`-free means (a test name prefix `cap__C_04__core__round_trip__…` is simplest and dependency-free) — decide; TS tests tagged with `test("[cap C.04 pdf round-trip] …")`.
- `cargo xtask capabilities [--check]`: runs tagged tests (nextest filter / Vitest `-t`), collects passing evidence, computes the best proven status per (row, surface), regenerates `docs/capabilities/README.md` (table per group with status icons and links to tests), and with `--check` fails if the committed README claims more than the evidence or if the manifest references unknown IDs.
- Seed evidence from existing tests: open/read rows (B2c), `SetInfo`/`RotatePage` write + round-trip (B3c), generation rows (B5) if merged.
- Full-lane job running `--check`.

**Out of scope**

- Web/desktop column evidence (Playwright tags): added by web/desktop rounds using the same convention.

## Tasks

- [ ] Extract 06 §3's operations into `manifest.toml` (script in `tools/`, then hand-edit target rounds from the plans)
- [ ] Choose and document the tagging convention (Rust and TS)
- [ ] Implement `cargo xtask capabilities` with evidence collection and README generation
- [ ] Tag existing tests; generate the first README
- [ ] Add to the full lane; document in `CLAUDE.md` and full-editor §3 (parent updates that plan)

## Acceptance criteria

- [ ] Every operation in 06 §3 has exactly one manifest row with a stable ID
- [ ] `cargo xtask capabilities --check` passes on `main` and fails when a README row is edited to claim an unproven status or a tagged test is made to fail
- [ ] The generated README shows at least: page count/outline read (core), `SetInfo`/`RotatePage` round-trip (core)
- [ ] Runs in the full lane

## Post-merge checks

None.

## Tests

Unit tests for status aggregation and README generation; an integration test with a fake manifest and fake test results.

## Docs to update

- [ ] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the capability-matrix item
- [ ] `docs/README.md` (new `docs/capabilities/` folder), `CLAUDE.md`

## Risks and open questions

- Test-name tagging is brittle but dependency-free; a proc-macro is cleaner but adds a crate — decide in the PR.
- 06 §3's list will change; IDs are never reused (like warning codes).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
