# Code that stays easy to change

These rules keep lamina's code easy to read, change and reuse as it grows. They apply to `crates/*`, `packages/*`, `apps/*` and `tools/*`, and they extend [code-organization.md](code-organization.md), [rust.md](rust.md) and [typescript.md](typescript.md). The [quality plan](../plans/quality.md) records the review that led to them and the work that brings today's code in line.

**MUST**/**SHOULD** are rules. *Recommendation* marks advice.

## One home per concept

- **MUST:** search the workspace before writing a helper, type or table. If one exists, use it; if it is wrong, fix it where it lives.
- **MUST:** a concept used by two crates or packages lives in the lower one. Examples:
  - one `Matrix` in `lamina-model`;
  - one PDF lexer in `lamina-syntax`;
  - one error-propagation policy per meaning in `lamina_cos::error`.
- **MUST:** one crate interprets each PDF attribute. Lower layers hold the raw object. Page attributes belong to `lamina-model`. Each crate's README names what it owns.
- **MUST:** a list that crosses Rust, WASM and TypeScript is defined once and generated for the other side. That covers codes, enum values, tag names and version strings. Until generation lands, a test pins it.
- **MUST:** logic that exists in both languages (did-you-mean, for example) is pinned by one shared fixture, and both test suites run that fixture.

## Size

- **MUST:** a Rust function is at most 100 lines, a limit `clippy::too_many_lines` already enforces. A Rust module is at most 800 lines of non-test code (comments, blank lines and `#[cfg(test)]` blocks don't count).
- **MUST:** a TypeScript function has a cognitive complexity of at most 15 and is at most 60 lines.
- **MUST:** budgets only go down. Code over a budget today is listed in a baseline file, and new code may not join it. The count of `#[expect]` on size lints may not grow. Generated files (`generated/`) are exempt.

## Public surface

- **MUST:** each public item of a library has one public path. The `lamina` façade never re-exports two items with the same name.
- **MUST:** shipped code contains no test code. Test-only APIs never appear in a public surface, and neither do test hooks (`__`-prefixed methods, logs kept for tests).
- **MUST:** test helpers used by two crates live in `lamina-test-fixtures`; in TypeScript they live in `@laminapdf/test-utils`. No `#[path]` and no relative import reaches into another crate or package.
- **MUST:** TypeScript throws only `Error` subclasses. Every error that crosses a boundary is the ADR 0026 object, built by one serialiser per language.

## Names and comments

- **MUST:** code and test files are named for what they do, never for the review or todo that created them.
- **MUST:** comments explain why the code is the way it is: a spec clause, an invariant or a workaround. Todo IDs belong in commit messages and plans, not in `src/`.

## How it's checked

- **Rust:** `cargo xtask check maintainability`, part of `cargo xtask check`, checks:
  - the module budget against its baseline;
  - the `#[expect]` count;
  - cross-crate `#[path]` includes;
  - error-policy matches outside `lamina_cos::error`;
  - lexers outside `lamina-syntax`;
  - shared test helpers outside the fixtures crate;
  - todo IDs in `src/`;
  - duplicate public type names (an allow-list with reasons);
  - colliding façade re-exports.

  The workspace adds `clippy::cognitive_complexity` and `unreachable_pub`.
- **TypeScript:** `cargo xtask check ts` runs:
  - Biome's complexity and function-length rules, plus `useThrowOnlyError`;
  - dependency-cruiser, for cycles and cross-package imports;
  - jscpd, for duplicated blocks;
  - knip, for unused exports and files;
  - TypeDoc's check for undocumented exports.
- **Rollout:** the [Round Q todos](../plans/quality.md#6-todos) add these checks and then clear the baselines.
- **Review:** reviewers judge "one home per concept". The `/code-review` standards axis reads this file.
