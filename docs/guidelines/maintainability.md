# Code that stays easy to change

These rules keep Lamina PDF's code easy to read, change and reuse as it grows. They apply to `crates/*`, `packages/*`, `apps/*` and `tools/*`, and they extend [code-organization.md](code-organization.md), [rust.md](rust.md) and [typescript.md](typescript.md). The [quality plan](../plans/quality.md) records the review behind them and the todos that bring today's code in line; where a rule's tooling doesn't exist yet, the rule names the todo that builds it.

**MUST**/**SHOULD** are rules.

## One home per concept

- **MUST:** search the workspace before writing a helper, type or table. If one exists, use it; if it is wrong, fix it where it lives.
- **MUST:** a concept used by two crates or packages lives in the lower one. Examples:
  - one `Matrix` in `laminapdf-model`;
  - one PDF lexer in `laminapdf-syntax`;
  - one function per error-propagation policy (such as `tolerant`) in `laminapdf_cos::error`.
- **MUST:** one crate interprets each PDF attribute, and lower layers hold the raw object. Page attributes belong to `laminapdf-model`. Each crate's README names what it owns.
- **MUST:** a list that crosses Rust, WASM and TypeScript (codes, enum values, tag names, version strings) has one source, and a generator writes the copies in the other languages. Until the generator exists, a test pins the list.
- **MUST:** logic that exists in both languages (for example, did-you-mean suggestions) is pinned by one shared fixture, and both test suites run it.

## Size

- **MUST:** a Rust function is at most 100 lines; `clippy::too_many_lines` already enforces this. A Rust module is at most 800 lines of non-test code. Comments, blank lines and `#[cfg(test)]` blocks don't count.
- **MUST:** a Rust function stays within clippy's `cognitive_complexity` threshold (the default, 25).
- **MUST:** a TypeScript function has a cognitive complexity of at most 15 and is at most 60 lines. Biome enforces both; test files are exempt from the length limit only, because a `describe` callback holds a whole suite ([typescript.md](typescript.md#lint-format-test)).
- **MUST:** the baseline of code over a budget only shrinks. New code must not join it, and the count of `#[expect]` on size lints must not grow. Generated files (`generated/`) are exempt.

## Public surface and tests

- **MUST:** each public item of a library has one public path. The `laminapdf` façade never re-exports two items with the same name.
- **MUST:** shipped code contains no test code. Test-only APIs never appear in a public surface, and `src/` contains no test hooks (`__`-prefixed methods, logs kept for tests).
- **MUST:** shared test helpers have one home:
  - Rust helpers used by two crates live in `laminapdf-test-fixtures` (created by todo Q2c).
  - TypeScript helpers live in `@laminapdf/test-utils` (`packages/test-utils`): the render contract suite, the browser harness and config, the Firefox workaround (`firefoxProvider`) and the repository root (`fromRoot`).

  No `#[path]` and no relative import reaches into another crate or package.

## Errors

- **MUST:** TypeScript throws only `Error` subclasses.
- **MUST:** every error that crosses a boundary is the ADR 0026 object, built by one serialiser per language. Boundaries are Rust to WASM, WASM to TypeScript, and a worker's messages.

## Names and comments

- **MUST:** code and test files are named for what they do, never for the review or todo that created them.
- **MUST:** comments explain why the code is the way it is: a spec clause, an invariant or a workaround. Todo IDs belong in commit messages and plans, not in `src/`.

## How it's checked

- **Rust:** `cargo xtask check maintainability` runs inside `cargo xtask check rust`, so on every code PR. It needs no build and takes about a second. It has seven checks:
  - `module-budget`: each `.rs` file under `crates/*/src/`, `tools/*/src/` and `apps/*/src-tauri/src/` holds at most 800 lines of non-test code. Tests, benches and examples have no module budget, and generated files (`generated/`, `*_generated.rs`) are exempt.
  - `size-expect`: counts each size lint named by an `#[expect]`, `#[allow]` or `#[cfg_attr(…)]` attribute (inner `#![…]` ones too). The size lints are `clippy::too_many_lines`, `cognitive_complexity`, `too_many_arguments` and `excessive_nesting`, and an attribute that names two counts twice. The check counts them in every Rust file under `crates/`, `tools/` and `apps/`, tests and examples included, generated files not, and fails when the total passes the baseline's.
  - `duplicate-type`: a `pub struct` or `pub enum` name defined in the `src/` of two crates, outside test code.
  - `facade-collision`: one name made public twice by the `laminapdf` façade's public modules (`lib.rs` and the `pub mod name;` files it declares), by a `pub use` or by a `pub` item defined there. It doesn't see what a glob (`pub use a::*`) brings in, and it doesn't follow an inline `pub mod name { … }`; the façade uses neither.
  - `error-policy`: a `matches!` that names `LimitExceeded` or `DataPending`, or an error or-pattern that lists one beside other errors (`Err(A | B)`, `e @ (A | B)`), in a crate's `src/`, outside test code and outside `crates/laminapdf-cos/src/error.rs`. That file decides which errors a read swallows: use one of its policies, or add one there. A match arm on one variant alone, such as one that collects `DataPending`'s ranges, is allowed.
  - `baseline-shrinks` and `allow-list-shrinks` compare the two record files below with their versions at the merge base with `main`.

  Two files record what exists today. `tools/xtask/maintainability-baseline.toml` lists the modules over the budget with their counts, and the number of size expects. `tools/xtask/maintainability-duplicates.toml` lists the duplicate names and façade collisions, each with a reason. Both only shrink. The check fails when the code goes past a record: a listed module grows by one line, a new module passes 800 lines, or a new expect or duplicate appears. It also fails when the code improves and the record doesn't, so lower or remove the entry in the same PR. Against the merge base, a baseline entry may not be added or raised, nor may the size-expect count. A new allow-list entry, or one with more homes, is a warning: a new duplicate is sometimes right, so a reviewer decides. When main's record files can't be read (no merge base in a shallow clone, or a file that no longer parses), the check warns that it couldn't compare.

  A line of non-test code holds something other than whitespace and comments, outside `#[cfg(test)]` items. The check reads each file with a small lexer rather than line by line:
  - Line comments, doc comments (`///`, `//!`) and block comments, nested ones included, are comments. A line with code and a comment counts.
  - The inside of a string, byte string, C string, raw string or character literal is code, so a non-blank line of a multi-line string counts. Braces, quotes and `//` inside a literal don't change the count, and a lifetime (`'a`) isn't read as a character literal.
  - A `#[cfg(test)]` attribute, or `#[cfg(all(…))]` with `test` among its arguments, marks every line from the attribute to the end of what it is on. That end is the first `;` outside brackets, or the `}` that closes its first braced body (an item's, a macro call's, a match arm's or a block's). On a field, a variant or a match arm, it is also the first `,`. A bracket that closes the enclosing list or block ends it too. Where the scan can't tell, it stops early, so the count errs towards more code.
  - A `#![cfg(test)]` makes its module test code: the whole file at the top, or an inline `mod name { … }`. A module declared by `#[cfg(test)] mod name;` is a test file, found where Cargo looks for it or at its `#[path]`.

  The workspace also turns on `clippy::cognitive_complexity` and `unreachable_pub` ([rust.md](rust.md#lints)). Later todos add their own bans to the check, each as one more entry in its list of checks (`tools/xtask/src/maintainability.rs`):
  - Q2c: cross-crate `#[path]` and shared helpers outside the fixtures crate;
  - Q2e: error-policy matches outside `laminapdf_cos::error`;
  - Q2i: lexers outside `laminapdf-syntax`;
  - Q2t: todo IDs in `src/`.
- **TypeScript:** todo Q2b adds Biome's complexity, function-length and `useThrowOnlyError` rules and TypeDoc's check for undocumented exports to `cargo xtask check ts`. Todo Q2u adds three more, run by `cargo xtask check ts` through the root scripts `check:deps`, `check:duplicates` and `check:unused`:
  - dependency-cruiser (`.dependency-cruiser.json`) fails on an import cycle, or a relative import from one package or app into another, across `packages/` and `apps/`. Its known-violations list (`.dependency-cruiser-known-violations.json`) is empty, and it only shrinks: `check ts` fails while it lists a violation that no longer occurs.
  - jscpd (`.jscpd.json`) fails on any block of 10 lines or more repeated in any package's top-level `src/`, a new package's included (an example's own `src/`, such as the SDK's Worker example, is out of scope). It checks `src/` only for now. The SDK's two per-condition entries (`src/entry/node.ts` and `src/entry/workerd.ts`) are excluded: each binds the same three loader functions to its own runtime's loader, with that runtime in its doc comments.
  - knip (`knip.config.ts`) fails on an unused file, export or dependency. It reads each package's entries from its `exports` (`tools/ts/package-entries.mjs`), plus what runs without being exported: worker files, test files, the files a test runner loads, configs and examples. Test helpers aren't entries, so an export only its own helper file uses is reported. knip counts an export a test uses as used, and doesn't report a package entry's own exports.
- **Review:** the "one home per concept" rules are a reviewer's judgement; the `/code-review` standards axis reads this file.
