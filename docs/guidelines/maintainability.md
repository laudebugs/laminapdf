# Code that stays easy to change

These rules keep lamina's code easy to read, change and reuse as it grows. They apply to `crates/*`, `packages/*`, `apps/*` and `tools/*`, and they extend [code-organization.md](code-organization.md), [rust.md](rust.md) and [typescript.md](typescript.md). The [quality plan](../plans/quality.md) records the review behind them and the todos that bring today's code in line; where a rule's tooling doesn't exist yet, the rule names the todo that builds it.

**MUST**/**SHOULD** are rules.

## One home per concept

- **MUST:** search the workspace before writing a helper, type or table. If one exists, use it; if it is wrong, fix it where it lives.
- **MUST:** a concept used by two crates or packages lives in the lower one. Examples:
  - one `Matrix` in `lamina-model`;
  - one PDF lexer in `lamina-syntax`;
  - one function per error-propagation policy (such as `tolerant`) in `lamina_cos::error`.
- **MUST:** one crate interprets each PDF attribute, and lower layers hold the raw object. Page attributes belong to `lamina-model`. Each crate's README names what it owns.
- **MUST:** a list that crosses Rust, WASM and TypeScript (codes, enum values, tag names, version strings) has one source, and a generator writes the copies in the other languages. Until the generator exists, a test pins the list.
- **MUST:** logic that exists in both languages (for example, did-you-mean suggestions) is pinned by one shared fixture, and both test suites run it.

## Size

- **MUST:** a Rust function is at most 100 lines; `clippy::too_many_lines` already enforces this. A Rust module is at most 800 lines of non-test code. Comments, blank lines and `#[cfg(test)]` blocks don't count.
- **MUST:** a Rust function stays within clippy's `cognitive_complexity` threshold (the default, 25).
- **MUST:** a TypeScript function has a cognitive complexity of at most 15 and is at most 60 lines.
- **MUST:** the baseline of code over a budget only shrinks. New code must not join it, and the count of `#[expect]` on size lints must not grow. Generated files (`generated/`) are exempt.

## Public surface and tests

- **MUST:** each public item of a library has one public path. The `lamina` façade never re-exports two items with the same name.
- **MUST:** shipped code contains no test code. Test-only APIs never appear in a public surface, and `src/` contains no test hooks (`__`-prefixed methods, logs kept for tests).
- **MUST:** shared test helpers have one home:
  - Rust helpers used by two crates live in `lamina-test-fixtures` (created by todo Q2c).
  - TypeScript helpers live in `@laminapdf/test-utils` (todo Q2m).

  No `#[path]` and no relative import reaches into another crate or package.

## Errors

- **MUST:** TypeScript throws only `Error` subclasses.
- **MUST:** every error that crosses a boundary is the ADR 0026 object, built by one serialiser per language. Boundaries are Rust to WASM, WASM to TypeScript, and a worker's messages.

## Names and comments

- **MUST:** code and test files are named for what they do, never for the review or todo that created them.
- **MUST:** comments explain why the code is the way it is: a spec clause, an invariant or a workaround. Todo IDs belong in commit messages and plans, not in `src/`.

## How it's checked

- **Rust:** todo Q2a adds `cargo xtask check maintainability` to `cargo xtask check`, starting with the module budget and its baseline, the `#[expect]` count, duplicate public type names (an allow-list with reasons) and colliding façade re-exports. Later todos add their own bans to it:
  - Q2c: cross-crate `#[path]` and shared helpers outside the fixtures crate;
  - Q2e: error-policy matches outside `lamina_cos::error`;
  - Q2i: lexers outside `lamina-syntax`;
  - Q2t: todo IDs in `src/`.

  Q2a also switches on `clippy::cognitive_complexity` and `unreachable_pub` for the workspace.
- **TypeScript:** todo Q2b adds Biome's complexity, function-length and `useThrowOnlyError` rules and TypeDoc's check for undocumented exports to `cargo xtask check ts`. Todo Q2u adds three more, run by `cargo xtask check ts` through the root scripts `check:deps`, `check:duplicates` and `check:unused`:
  - dependency-cruiser (`.dependency-cruiser.json`) fails on an import cycle or a relative import into another package. Today's eight render-adapter test imports, which Q2m moves into a shared package, are in `.dependency-cruiser-known-violations.json`, and that list only shrinks.
  - jscpd (`.jscpd.json`) fails on any block of 10 lines or more repeated in `packages/*/src`. Tests aren't checked yet. The SDK's per-condition entries (`src/entry/*.ts`) are excluded: each binds the same three loader functions to its own runtime's loader, with that runtime in its doc comments.
  - knip (`knip.config.ts`) fails on an unused file, export or dependency. It reads each package's entries from its `exports`, plus the worker files, tests, configs and examples no `exports` can name.
- **Review:** the "one home per concept" rules are a reviewer's judgement; the `/code-review` standards axis reads this file.
