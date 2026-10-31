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
| [0004](0004-render-backend.md) | Render backend: `RenderBackend` trait (tiles in straight RGBA, `CancelToken`, `RenderLimits`), PDFium by default per surface (pinned and hashed), hayro as CI oracle with switch criteria, MuPDF prohibited, the two-parsers rule | Accepted |
| [0005](0005-document-model.md) | Document model: lazy COS over `ByteSource`, dirty overlay, typed views, `LaminaId`s | Accepted (generation strategy: own builder over the overlay, per the [`krilla-fit` spike](../../spikes/krilla-fit/README.md)) |
| [0006](0006-op-log-and-undo.md) | Op log, transactions, undo/redo and barrier ops | Accepted |
| [0007](0007-save-planner.md) | Save planner, `SaveMode` and `SaveConstraint` | Accepted |
| [0009](0009-web-client-architecture.md) | Web client: React 19 chrome in `@laminapdf/ui`, framework-free `@laminapdf/viewer`, one document worker per document, typed RPC on `@laminapdf/pdf/worker`, transferred `ImageBitmap`s, single-threaded v0.1 without COOP/COEP | Accepted |
| [0010](0010-platform-adapter.md) | Platform adapter: one `Platform` interface, `PlatformFile`, capability flags with web values, no platform checks in UI code | Accepted |
| [0011](0011-cloudflare-topology-and-privacy.md) | Cloudflare topology (static-assets Workers for staging and production, previews, production only from tags), the v0.1 privacy guarantees, security headers, telemetry and hosted-service rules | Accepted (login stack by amendment after H1) |
| [0015](0015-monorepo-layout-and-tooling.md) | Monorepo layout and tooling (Cargo + pnpm + xtask) | Accepted |
| [0016](0016-package-design.md) | Package design: ESM-only, export conditions, base and text WASM modules, per-module size budgets | Accepted |
| [0017](0017-versioning-and-release.md) | Versioning and release: one lockstep version for every crate, `@laminapdf/wasm`, `@laminapdf/pdf` and the render packages; Changesets with `cargo xtask version sync`; `0.x` and `-alpha.N`; `v<version>` tags; the release lane with its nightly guard and `verify-install`, no publish step | Accepted (versioning; publishing part in P1) |
| [0018](0018-test-strategy.md) | Test strategy: tiers, CI lanes, corpora, fuzzing, flaky tests | Accepted |
| [0019](0019-supply-chain-and-licence-hygiene.md) | Supply-chain and licence hygiene (cargo-deny, npm licence check, Dependabot, SBOM, fixture provenance) | Accepted |
| [0023](0023-content-stream-reading.md) | Content-stream reading: the `lamina-content` crate (tokenizer, visitor), tolerance rules, operator and XObject-depth limits | Accepted |
| [0026](0026-error-and-diagnostic-contract.md) | Error and diagnostic contract: one JSON shape, `kind`s, `hint`/`docsUrl`, no document text in messages, `explain` pages, `LaminaError` | Accepted |
| [0027](0027-coordinates-units-and-page-indexing.md) | Coordinates, units and page indexing: top-left points of the displayed CropBox, raw user space named as such, `Pt`, 0-based `index` + 1-based `number` | Accepted |
| [0028](0028-facade-rules-jobs-and-cancellation.md) | Façade rules, jobs, cancellation and progress: UniFFI-compatible rules, lock in the binding edge, `Job::step(Budget)` in work units, `CancelToken` | Accepted |
| [0029](0029-mobile-bindings-and-packaging.md) | Mobile bindings and packaging: UniFFI `=0.32.2`, `crates/lamina-ffi` edge crate, idiomatic Swift/Kotlin layers, SwiftPM and Maven Central AARs, 16 KB pages, `mobile-release` profile | Accepted |
