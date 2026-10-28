# Architecture Decision Records

One file per significant, hard-to-reverse decision, named `NNNN-short-title.md` (e.g. `0001-core-language.md`).

Template:

```markdown
# NNNN — Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
What forces are at play? What problem are we solving?

## Decision
What we decided.

## Consequences
What becomes easier or harder because of this.

## Alternatives considered
Options rejected and why.
```

Numbers follow the [architecture plan §3](../plans/architecture.md#3-decisions-to-record-adrs), so they are not written in order.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-core-language-and-licence.md) | Core language (Rust), licence (MIT OR Apache-2.0) and the bytes-in/bytes-out rule | Accepted |
| [0002](0002-binding-strategy.md) | Binding strategy: wasm-bindgen + tsify; UniFFI later; napi-rs and Component Model deferred | Accepted |
| [0003](0003-reader-tolerance-and-limits.md) | Reader tolerance policy, warning codes and `Limits` defaults | Accepted |
| [0005](0005-document-model.md) | Document model: lazy COS over `ByteSource`, dirty overlay, typed views, `LaminaId`s | Accepted (generation strategy Proposed until the `krilla-fit` spike) |
| [0006](0006-op-log-and-undo.md) | Op log, transactions, undo/redo and barrier ops | Accepted |
| [0007](0007-save-planner.md) | Save planner, `SaveMode` and `SaveConstraint` | Accepted |
| [0015](0015-monorepo-layout-and-tooling.md) | Monorepo layout and tooling (Cargo + pnpm + xtask) | Accepted |
| [0018](0018-test-strategy.md) | Test strategy: tiers, CI lanes, corpora, fuzzing, flaky tests | Accepted |
| [0019](0019-supply-chain-and-licence-hygiene.md) | Supply-chain and licence hygiene (cargo-deny, npm licence check, Dependabot, SBOM, fixture provenance) | Accepted |
