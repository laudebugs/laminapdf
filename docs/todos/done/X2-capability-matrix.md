# X2 — Capability matrix generator: ~180 operations, test-backed statuses

| Field | Value |
| --- | --- |
| ID | `X2` |
| Status | `done` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting), [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) |
| Round / milestone | Cross-cutting (before the editor rounds; useful from M1) |
| Depends on | [B3c](./B3c-save-planner-and-roundtrip.md) |
| Blocks | [E1a](../ready/E1a-editing-foundation-feature-plan.md), [E1f](../ready/E1f-metadata-ui-capability-page.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) (tests are the evidence), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (xtask) |
| Branch | `build/x2-capability-matrix` |
| PR title | `build(xtask): test-backed capability matrix for the ~180 operations` |
| Size | L (≈ 700–1,100 lines incl. the manifest) |
| PR | PR_PLACEHOLDER |

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

- [x] Extract 06 §3's operations into `manifest.toml` (script in `tools/`, then hand-edit target rounds from the plans). *Deviation:* the one-off extraction script was not kept in `tools/`; instead `cargo xtask check docs` parses 06 §3 on every PR and fails unless every operation has exactly one manifest row (same group, same name) and every row is in 06. 06 §3 has **169** operations (the plans' "~180")
- [x] Choose and document the tagging convention (Rust and TS): a doc-comment line on the test, `/// capability: C.08 core round-trip` (no rename, no proc-macro, no dependency; test names keep reading as behaviour per rust.md), on `#[test]` fns in `src/` or `tests/<name>.rs`; TS `test("[cap C.04 pdf round-trip] …")`. *Deviation:* there is no TS package yet, so a TS tag is an error ("not collected yet") rather than run with Vitest `-t`; the first package that tags a test adds the runner
- [x] Implement `cargo xtask capabilities` with evidence collection and README generation (`tools/xtask/src/capabilities.rs` + `capabilities/{manifest,tags,matrix}.rs`; one `cargo test -p <crate> --lib|--test <name> -- <fns>` per target, libtest output parsed; plain `cargo test`, not nextest)
- [x] Tag existing tests; generate the first README: 25 tags on 22 tests in `lamina-model`, `lamina-cos`, `lamina-save`. core round-trip: C.08 rotate, J.01 title (`SetInfo`; Info only, XMP sync comes with E1), G.05 remove encryption; write: K.04 remove unused objects (full rewrite); read: C.04 (page count), C.08, C.09, C.11, C.14, F.05, F.08, F.09, G.01, G.03, G.04, G.11, I.01 (outline), I.04, I.10, J.01, J.02, K.10
- [x] Add to the full lane (`capabilities` job in `full.yml`, in `gate`'s needs); document in `CLAUDE.md`, full-editor §3 and ADR 0018's implementation note

## Acceptance criteria

- [x] Every operation in 06 §3 has exactly one manifest row with a stable ID (enforced by `check docs`)
- [x] `cargo xtask capabilities --check` passes on `main` and fails when a README row is edited to claim an unproven status or a tagged test is made to fail (both checked by hand; an overclaim also fails `cargo xtask check docs`, which needs no test run and so covers docs-only PRs)
- [x] The generated README shows at least: page count/outline read (core: C.04, I.01), `SetInfo`/`RotatePage` round-trip (core: J.01, C.08)
- [x] Runs in the full lane

## Post-merge checks

None.

## Tests

Unit tests for status aggregation and README generation; an integration test with a fake manifest and fake test results.

## Docs to update

- [x] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the capability-matrix item
- [x] `docs/README.md` (new `docs/capabilities/` folder), `CLAUDE.md`

## Risks and open questions

- Test-name tagging is brittle but dependency-free; a proc-macro is cleaner but adds a crate — decide in the PR. *Decided:* a doc-comment tag (`/// capability: …`), dependency-free and without renaming tests. A renamed or moved test changes the generated page, so `--check` catches it; a deleted one drops its claims, so `check docs` catches the stale page.
- 06 §3's list will change; IDs are never reused (like warning codes). *Done:* the manifest's `retired` list; a retired ID can be neither a row nor a tag.
- Core `write`/`round-trip` evidence is lamina's side of the round-trip suite; the qpdf/pdf.js/PDFium half (`cargo xtask roundtrip`) is a separate full-lane job, not linked to individual rows yet.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
