# Rust conventions

These apply to all product Rust code in `crates/`, `apps/*/src-tauri` and `tools/`. They come from the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), the Cargo and Clippy books, the `rust-best-practices` skill (Apollo GraphQL's [Rust Best Practices handbook](https://github.com/apollographql/rust-best-practices)), and what the [`pdf-basics` spike](../../spikes/pdf-basics/rust/README.md) enforced in practice. Layout and crate boundaries are covered in [`code-organization.md`](code-organization.md).

**MUST**/**SHOULD** are rules. *Recommendation* marks advice.

**MUST:** load the `rust-best-practices` skill before writing or reviewing Rust code. For Tauri code (`apps/desktop/src-tauri`, commands, IPC, capabilities, plugins), also load the `tauri-v2` skill. This guide takes precedence where they differ.

## Toolchain, edition, MSRV

- **MUST:** use edition **2024** for every crate. Set it once in `[workspace.package]` and inherit it with `edition.workspace = true`.
- **MUST:** pin the development toolchain and targets in `rust-toolchain.toml` (stable channel; components `rustfmt`, `clippy`; targets `wasm32-unknown-unknown`, `aarch64-apple-ios`, `aarch64-linux-android`, plus any others we ship). CI uses the same file.
- **MUST:** declare `rust-version` (the MSRV) in `[workspace.package]`, and verify it in CI with a job on that exact toolchain. Clippy reads it, so it won't suggest newer APIs ([Cargo: rust-version](https://doc.rust-lang.org/cargo/reference/rust-version.html), [Clippy `msrv`](https://doc.rust-lang.org/clippy/lint_configuration.html)).
- **MUST:** keep the **N-2 policy** (adopted with the first registry release, 0.2.0-alpha.1): the MSRV is no newer than N-2, i.e. at least two stable releases behind the latest stable (with Rust 1.99 current, `rust-version` is at most 1.97). Raise it only when a newer Rust feature or a dependency's MSRV requires it, never past N-2, and treat the bump as a minor-version change noted in the changelog. The Cargo book asks projects to publish their policy and keep `rust-version` in line with it.

## Lints

Configure lints once, in the root `Cargo.toml`. Every member opts in with `[lints] workspace = true` (requires Rust ≥ 1.74). This is the spike's set, lifted to the workspace and tightened (for example `pedantic`, `indexing_slicing`, `todo`/`unimplemented`, `allow_attributes`); lint names checked against Clippy 0.1.99:

```toml
[workspace.lints.rust]
unsafe_code = "deny"            # core crates additionally #![forbid(unsafe_code)]
missing_docs = "deny"
future_incompatible = { level = "warn", priority = -1 }
nonstandard_style = { level = "deny", priority = -1 }
unused_qualifications = "warn"
unreachable_pub = "warn"        # `pub` in a private module is `pub(crate)`: the public surface is only what lib.rs exports

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }   # enforced: CI's -D warnings turns these into errors
must_use_candidate = "allow"    # pedantic allow-list: each entry says why (here: #[must_use] on every getter is noise)
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"       # spike had "warn"; product core denies
redundant_clone = "deny"
undocumented_unsafe_blocks = "deny"
dbg_macro = "deny"
todo = "deny"                   # unfinished code never merges; keep todo!/unimplemented! on your branch
unimplemented = "deny"
allow_attributes = "deny"       # outer #[allow] is rejected: use #[expect] (inner #![allow] is not covered)
allow_attributes_without_reason = "deny"   # every #[allow]/#[expect] states why
cognitive_complexity = "warn"   # a nursery lint (not in `all` or `pedantic`), so it is named here; threshold: the default 25
```

`clippy.toml`:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
allow-dbg-in-tests = true
# Output must be byte-deterministic (ADR 0001): RandomState iteration order must never reach output.
# Use BTreeMap/BTreeSet, IndexMap/IndexSet or a fixed-seed hasher.
disallowed-types = [
  { path = "std::collections::HashMap", reason = "random iteration order; use BTreeMap, IndexMap or a fixed hasher" },
  { path = "std::collections::HashSet", reason = "random iteration order; use BTreeSet, IndexSet or a fixed hasher" },
]

# Keep this paragraph last: tools/xtask/clippy-examples/clippy.toml copies everything above it.
too-many-lines-threshold = 100
```

- **MUST:** Clippy passes with `-D warnings`, so `pedantic` is enforced, not advisory. A pedantic lint that is noise for the whole workspace goes on the allow-list in `[workspace.lints.clippy]` with a comment (lint-specific entries at the default priority 0 override the `priority = -1` group). Two runs, because cfg-gated code is only linted when it is compiled:
  - native: `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - wasm32: `cargo clippy --target wasm32-unknown-unknown --locked -p laminapdf-core -p laminapdf-syntax -p laminapdf-wasm --lib -- -D warnings` (the core and WASM crates only, with default features; the Tauri crate, xtask and native-only features don't build for wasm32).
- **MUST:** code stays within the size limits of the [maintainability guideline](maintainability.md#size). A function is at most 100 lines (`clippy::too_many_lines`, threshold set in `clippy.toml`) and within `clippy::cognitive_complexity`'s default of 25. A module under a crate's, tool's or Tauri app's `src/` holds at most 800 lines of non-test code. `cargo xtask check maintainability` counts those lines and fails on a module that grows past its baseline or a new module over 800 lines. It also fails on a new size lint named by an `#[expect]`, `#[allow]` or `cfg_attr` (`too_many_lines`, `cognitive_complexity`, `too_many_arguments`, `excessive_nesting`); the [maintainability guideline](maintainability.md#how-its-checked) has the method. Split the code rather than silence the lint.
- **MUST:** `cargo fmt --check` passes. `rustfmt.toml` keeps `max_width = 120` and only stable options (`group_imports`/`imports_granularity` need nightly, so we don't require them).
- **MUST:** silence a lint locally with `#[expect(lint, reason = "…")]`, not `#[allow]` (`allow_attributes` rejects outer `#[allow]`; `allow_attributes_without_reason` requires the `reason` on both). `expect` warns once the lint stops firing, so stale exceptions get cleaned up. Workspace-wide `allow`s need a comment and a PR discussion.
- **Integration test crates:** Clippy's `allow-*-in-tests` only covers `#[test]` functions and `#[cfg(test)]` modules, not shared helpers in `tests/common`. Put a file-level `#![expect(clippy::unwrap_used, clippy::panic, reason = "test helpers: a panic is a test failure")]` at the top of each test file (use an inner `#![allow(…, reason = "…")]` only when `expect` would be unfulfilled in some test crates, as the spike found; `allow_attributes` doesn't cover inner attributes, so review these by hand).

## Unsafe

- **MUST:** core crates (parsing, model, writing, operations) contain no `unsafe`. They state this with `#![forbid(unsafe_code)]` in `lib.rs`.
- **MUST:** `unsafe` is allowed only in edge crates that need it (FFI or a C-ABI shim, WASM allocator exports), and only in a module that opts in with `#![expect(unsafe_code, reason = "…")]`, as the spike's `wasm.rs` did.
- **MUST:** every `unsafe` block has a `// SAFETY:` comment proving its preconditions, and every `unsafe fn` or trait has a `# Safety` doc section (`undocumented_unsafe_blocks`, `missing_safety_doc`).
- **SHOULD:** a new `unsafe` block gets a second reviewer. Run the tests under Miri where the code allows it.

## Errors

- **MUST:** library crates return `Result<T, CrateError>` with an error type derived via [`thiserror`](https://docs.rs/thiserror). Errors are `Debug + Display + std::error::Error + Send + Sync + 'static` ([C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err)).
- **MUST:** public error enums are `#[non_exhaustive]`. Variants carry structured context (byte offset, object number, limit), not formatted strings. Display messages are lowercase with no trailing period and never contain text read from the input or given by the caller ([ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) §4). The exception is `ErrorKind`: ADR 0026 §2 makes its nine kinds a closed set that bindings match exhaustively, so it is not `#[non_exhaustive]`, and adding a kind is a breaking change that amends the ADR.
- **MUST:** every error and warning type has `code()` (from `fixtures/codes.json`), `kind()` as an exhaustive match over its variants (no wildcard arm, no default kind; wrappers delegate to the wrapped error's `kind()`) and `location()` (ADR 0026 §1.1, positions tagged with `Position`). A unit test with one sample per variant checks `code()` and `kind()` against `codes.json`.
- **MUST NOT:** library code uses `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` or unchecked indexing/slicing (all denied by the lint table above). Use `?`, `let … else`, `.get()`, `ok_or(…)` and `checked_*` arithmetic. `unreachable!` is allowed only with a comment that proves the invariant. If it can be reached from input, it's a bug.
- **MUST NOT:** library APIs return `anyhow::Error` or `Box<dyn Error>`. Use `anyhow` only in binaries, xtask and tests.
- **SHOULD:** derive `Clone + PartialEq` on errors when the payload allows it, so tests can assert exact variants (`assert_eq!(err, Error::LimitExceeded { limit: Limit::StreamDecoded, at: None })`).
- **SHOULD NOT** expose another crate's error type through `#[from]` in a public variant. That makes the dependency's semver part of ours. Map it to our own variant and keep the source with `#[source]` only if it's a stable, public dependency.
- **MUST:** every count, length or offset read from a file is untrusted. Bound allocations by the bytes actually present, and give recursion a depth limit. Loops over input-supplied counts must not allocate based on the count alone. See [research ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned).
- Recoverable irregularities (a missing header, a repaired xref) are reported as **warnings** on the result, not as errors and not silently.

## API design

- **MUST:** items are private by default, and `pub(crate)` for crate-internal sharing. `lib.rs` re-exports the public API (`pub use`). Hide implementation modules.
- **MUST:** follow the [API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html) for public items. Key items: naming (C-CASE, `as_`/`to_`/`into_` conversions C-CONV, no `get_` prefix C-GETTER), common traits (`Debug`, `Clone`, `PartialEq`, `Default` where they make sense; C-COMMON-TRAITS, C-DEBUG), validated newtypes over primitives (C-NEWTYPE), builders for complex construction (C-BUILDER), sealed traits for traits we must be able to extend (C-SEALED).
- **SHOULD:** take borrowed inputs (`&[u8]`, `&str`, `impl AsRef<[u8]>`), and return owned values or values borrowed from the input. Only take ownership when you store the value.
- **SHOULD:** prefer generics (static dispatch) in hot paths, and use `dyn Trait` at API boundaries that need heterogeneity or a smaller WASM size.
- **SHOULD:** document thread-safety. If a type is deliberately `!Send`/`!Sync` (the spike's `Rc`/`RefCell` caches), say so in its docs.
- **MUST:** a `#[deprecated]` item gets an entry in [`docs/renames.json`](../renames.json) naming what replaces it, and so does a public item renamed without a deprecation. The packages ship that file, so an agent holding an old name finds the new one; `cargo xtask check docs` fails on a deprecation without an entry.
- **MUST:** Cargo features are additive and named without `use-`/`with-` (C-FEATURE). `cargo hack check --feature-powerset` (or an equivalent) runs in CI once a crate has more than two features.

## Tests

- **MUST:** unit tests sit in `#[cfg(test)] mod tests` at the bottom of the file. Integration tests in `tests/<topic>.rs` use the public API only. A crate's own shared helpers go in `tests/common/mod.rs`; helpers several crates need live in `laminapdf-test-fixtures` ([`code-organization.md` §6](code-organization.md#6-tests-and-fixtures)).
- **MUST:** test names read as behavior. Group them by the unit under test: `mod parse_object { #[test] fn rejects_nesting_deeper_than_max_depth() … }` or `open_should_repair_when_xref_offset_is_wrong`. No `test_` prefixes, no `happy_path`.
- **SHOULD:** check one behavior per test. Use [`rstest`](https://docs.rs/rstest) cases for tables of inputs and [`insta`](https://insta.rs) for snapshotting large text or `Debug` output (never for byte-exact PDFs; those are golden files in `fixtures/golden/`, rewritten only by an explicit regenerate command, see [`code-organization.md` §6](code-organization.md#6-tests-and-fixtures)).
- **MUST:** every public example in docs is a doc test that compiles and runs (`?` with a hidden `# Ok::<(), Error>(())`, C-QUESTION-MARK).
- **MUST:** parsers have robustness tests (truncation, byte flips, token soup) with a time budget, enforced by the test runner's timeout (nextest `slow-timeout` in `.config/nextest.toml`), never by asserting elapsed time ([ADR 0018](../decisions/0018-test-strategy.md)). Fuzz targets (`cargo fuzz`) live in the root `fuzz/` folder (OSS-Fuzz layout, one target per parser entry point), which is its own workspace and excluded from the root one ([ADR 0015](../decisions/0015-monorepo-layout-and-tooling.md)).

## Documentation

- **MUST:** `lib.rs` opens with `//!` docs that say what the crate does, with a runnable example (C-CRATE-DOC).
- **MUST:** public functions document `# Errors` (when they return `Result`), `# Panics` (should never be needed in library code) and `# Safety` (C-FAILURE). Link related items with intra-doc links (`[`Document`]`). `rustdoc::broken_intra_doc_links` is denied in CI (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`).
- **SHOULD:** use `//` comments for *why*: spec references (`// ISO 32000-2 §7.5.8.4`), invariants and workarounds. `TODO(#issue)` only.

## Dependencies

- **MUST:** declare versions once in `[workspace.dependencies]`. Members use `dep = { workspace = true }`.
- **MUST:** check each new dependency's licence. Allowed: MIT, Apache-2.0, BSD, Zlib, ISC, Unicode, 0BSD. No GPL, LGPL or AGPL in shipped crates (see [research ch. 05 §8.4](../research/pdf/05-creating-pdfs-from-code.md#84-licensing-traps-to-avoid)). Add a comment with the licence next to it, as the spike does. *Recommendation:* enforce this with [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) (`licenses`, `bans`, `advisories`) in CI.
- **SHOULD:** core crates use `default-features = false` on dependencies and enable only what they need. Watch WASM size (the spike's release WASM was 74 KB gzipped).

## Performance

- **SHOULD:** borrow, don't clone (`redundant_clone` is denied). Avoid intermediate `collect()`. Measure with `--release` and `cargo bench` (Criterion) before optimizing.
- **SHOULD:** release profile for WASM: `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true` (the spike's settings). Set it per profile in the root manifest.

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and its [checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
- Cargo Book: [Workspaces (`[workspace.lints]`, inheritance)](https://doc.rust-lang.org/cargo/reference/workspaces.html), [Features](https://doc.rust-lang.org/cargo/reference/features.html), [`rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html)
- Clippy: [Usage](https://doc.rust-lang.org/clippy/usage.html), [Lint configuration](https://doc.rust-lang.org/clippy/lint_configuration.html), [Lint list](https://rust-lang.github.io/rust-clippy/master/)
- [rustdoc lints](https://doc.rust-lang.org/rustdoc/lints.html), [Rust style guide](https://doc.rust-lang.org/style-guide/)
- Apollo GraphQL, [Rust Best Practices](https://github.com/apollographql/rust-best-practices). This is the source of the `rust-best-practices` skill: ch. 2 linting, ch. 4 errors, ch. 5 testing, ch. 8 documentation.
- Workspace lint tables in practice: [Biome `Cargo.toml`](https://github.com/biomejs/biome/blob/main/Cargo.toml), [Typst `Cargo.toml`](https://github.com/typst/typst/blob/main/Cargo.toml)
- Lamina PDF spike: [`spikes/pdf-basics/rust/`](../../spikes/pdf-basics/rust/README.md) (`Cargo.toml`, `clippy.toml`, `scripts/check-targets.sh`)
