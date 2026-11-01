# Code examples readers can copy

An example is code a reader will paste into their own app, so it must be code we would merge. These rules apply to:

- `packages/sdk/examples/**`;
- `crates/lamina/examples/**`;
- every file the docs site's `Snippet` component may show.

The [quality plan](../plans/quality.md) explains why they exist, and [docs-writing.md](docs-writing.md) covers the page around the code.

**MUST**/**SHOULD** are rules. *Recommendation* marks advice.

## What an example is

- **MUST:** a page shows a file from `examples/`, never from `test/`. A test may import an example; an example never imports a test.
- **MUST:** an example does one thing and reads top to bottom. A reader could paste it into an app and run it unchanged.
- **MUST:** every example is shown on some page, or is listed in `examples/UNUSED.md` with the reason.
- **MUST:** a task shown in both TypeScript and Rust has the same name in each (`links-and-outlines.ts`, `links_and_outlines.rs`). Both files open with the same first header sentence.

## How it's written

- **MUST:** the file opens with a comment that says what it shows. That comment mentions no ADR or todo IDs, repository paths or test vocabulary such as "golden" or "twin".
- **MUST:** examples import only public entry points:
  - TypeScript: `@laminapdf/sdk` and `@laminapdf/sdk/worker`;
  - Rust: `lamina::…`.

  They never import from `src/`, `test/`, `fixtures/` or another package's path, and never from `lamina_<crate>` directly.
- **MUST:** the caller supplies test scaffolding. Fixed file IDs, dates and producers come from the caller, and the tests pass them in. Checks of the output belong in the tests.
- **MUST:** TypeScript examples free what they create with `using` (documents and builders implement `Symbol.dispose`).
- **MUST:** layout numbers have names (`MARGIN`, `ROW_HEIGHT`), or come from the API (the page width comes from the page size).
- **MUST:** functions are at most 40 lines, and files at most 100.
- **SHOULD:** errors are handled the way an app would handle them. Return the caller's errors as values and let everything else throw. Avoid noisy wrappers.

## Reviewer checklist

- Does it do one thing, top to bottom?
- Could a reader paste it into their app unchanged?
- Are the numbers named?
- Is everything it creates freed?
- Do the TypeScript and Rust versions match?
- Does the header say what it shows, in plain words?

## How it's checked

- **`cargo xtask check docs-snippets`**, part of `cargo xtask check docs`, checks:
  - the header comments;
  - the `test/` ban;
  - unused examples;
  - pair names;
  - internal imports in Rust;
  - determinism scaffolding;
  - file length.
- **A Biome override on `packages/sdk/examples/**`** forbids non-null assertions, `any`, internal imports, and functions over 40 lines.
- **`cargo xtask clippy-examples`** runs clippy on the Rust examples with a 40-line function limit, and denies discarding a `#[must_use]` value with `let _ =`.

All three warn until [Q1l](../todos/ready/Q1l-docs-checks-strict.md), then fail. The reviewer checklist covers what the tools can't judge.
