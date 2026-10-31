# Stability policy (draft)

**Draft, not yet in force.** It describes what lamina promises once it is on a registry; it is published with the first registry pre-release ([P2](todos/ready/P2-first-registry-prerelease.md)). Versioning itself is [ADR 0017](decisions/0017-versioning-and-release.md).

lamina is one library with one version: every crate and the npm packages `@laminapdf/wasm`, `@laminapdf/pdf`, `@laminapdf/render`, `@laminapdf/render-hayro` and `@laminapdf/render-pdfium` move together (lockstep). The version is `0.x`, currently `0.1.0-alpha.0`.

## What a version number tells you

- **`0.x` follows SemVer as Cargo reads it.** A breaking change bumps the minor version (`0.1 → 0.2`); features and fixes bump the patch (`0.1.0 → 0.1.1`). An upgrade within one `0.x` line never needs a code change.
- **Pre-releases (`-alpha.N`) promise nothing between each other.** Anything can change from `0.1.0-alpha.0` to `0.1.0-alpha.1`. Pin the exact version while you try an alpha.
- **Breaking** means: a public Rust item removed, renamed or changed in a way that stops existing code compiling or changes its documented behaviour; the same for the TypeScript API's exports and types; a raised minimum Rust version (MSRV), Node version or `compatibility_date`; a new `ErrorKind` (callers match on the nine kinds exhaustively); or output bytes that differ for the same input and options (outputs are byte-deterministic, and tests compare them byte for byte).
- **Not breaking:** new items, new optional options and fields, new error and warning codes (see below), documentation, performance, and fixes that make output conform to ISO 32000 where it did not (listed in the changelog).

## Promises that hold across every version, `0.x` included

- **Error and warning codes are never renamed or reused** with a new meaning ([ADR 0026](decisions/0026-error-and-diagnostic-contract.md), [ADR 0003 §2](decisions/0003-reader-tolerance-and-limits.md#2-warnings-stable-enumerated-codes)). A retired code stays listed in `fixtures/codes.json` with the version that retired it and keeps its [explain page](errors/README.md). Code on a `code` string keeps working; new codes can appear in any release, so handle unknown ones.
- **The error object's shape** is versioned by its schema id (`lamina.error/1`); a field is only added, never removed or retyped, within one schema version.
- **Op JSON is versioned** by its envelope (`{ "v": 1, "ops": [ … ] }`, [ADR 0006](decisions/0006-op-log-and-undo.md)). Adding an op or an optional field keeps `v`; changing or removing one bumps `v` and adds a migration, and every older `v` stays loadable. The frozen samples in `fixtures/ops/v1/` are tested on every PR.
- **Byte-identical output across platforms and runtimes** for the same input, options and version: Rust and every JavaScript runtime produce the same bytes (the release lane checks the quick start in each).

## What is not covered yet

- The low-level COS and model APIs re-exported by the façade are public but may change in any minor `0.x` release ([pdf-library §9, question 6](plans/pdf-library.md#9-open-questions)).
- Size budgets and performance targets are tracked, not promised.
- `cargo-semver-checks` and a public API report are enabled against the first published version ([P2](todos/ready/P2-first-registry-prerelease.md)); until then reviews check breaking changes by hand.

## Changelog

Every release lists its changes in [`CHANGELOG.md`](../CHANGELOG.md), generated from the changesets of each PR ([contributing: changesets](guidelines/contributing.md#changesets)). Breaking changes are listed first, under **Minor Changes** while the version is `0.x`.
