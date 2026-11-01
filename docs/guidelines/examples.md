# Code examples readers can copy

An example is code a reader will paste into their own app, so it must be code we would merge. These rules apply to:

- `packages/sdk/examples/**`;
- `crates/laminapdf/examples/**`;
- every file the docs site's `Snippet` component may show.

[docs-writing.md](docs-writing.md) covers the page around the code, and the [quality plan](../plans/quality.md) records why these rules exist.

**MUST**/**SHOULD** are rules.

## What an example is

- **MUST:** a page shows a file from `examples/`, never from `test/`. A test may import an example; an example never imports a test.
- **MUST:** an example does one thing and reads top to bottom. A reader could paste it into an app and run it unchanged.
- **MUST:** every example is shown on some page, or is listed in `examples/UNUSED.md` with the reason.
- **MUST:** a task shown in both TypeScript and Rust has the same name in each (`links-and-outlines.ts`, `links_and_outlines.rs`). Both files open with the same first header sentence.

## How it's written

- **MUST:** the file opens with a comment that says what it shows. That comment mentions no ADR or todo IDs, no repository paths and no test vocabulary such as "golden" or "twin".
- **MUST:** an example imports only public entry points:
  - in TypeScript, `@laminapdf/sdk` and `@laminapdf/sdk/worker`;
  - in Rust, `laminapdf::…`.

  It never imports from `src/`, `test/`, `fixtures/` or another package's path, and never from `laminapdf_<crate>` directly.
- **MUST:** an example contains no test scaffolding. Fixed file IDs, dates and producers are parameters, and the tests pass them in. Checks of the output belong in the tests.
- **MUST:** a TypeScript example frees what it creates with `using`; documents and builders implement `Symbol.dispose`.
- **MUST:** layout numbers have names (`MARGIN`, `ROW_HEIGHT`) or come from the API (the page width comes from the page size).
- **MUST:** functions are at most 40 lines, and files at most 100.
- **SHOULD:** handle errors the way an app would. Catch what the reader can act on (a wrong password: ask again) and let the rest throw. Don't wrap each call in a `try`/`catch` that only rethrows.

## Reviewer checklist

- Does it do one thing, top to bottom?
- Could a reader paste it into their app unchanged?
- Does it use only public entry points?
- Are the numbers named?
- Is everything it creates freed?
- Do the TypeScript and Rust versions match?
- Does the header say what it shows, in plain words?

## How it's checked

Three checks enforce these rules:

| Check | Runs in | Rules |
| --- | --- | --- |
| `cargo xtask check docs-snippets` | `cargo xtask check docs` (every PR, docs-only ones included) | `examples/` only; every example shown; matching pairs; header comment; Rust public imports; no test scaffolding; file length |
| A Biome override on `packages/sdk/examples/**`, and `cargo xtask biome-examples` for the changed ones | `cargo xtask check ts` | TypeScript public imports; no non-null assertions or `any`; function length and complexity |
| `cargo xtask clippy-examples`: clippy on `crates/laminapdf/examples/` with an examples-only `clippy.toml` (function length 40) and `let_underscore_must_use` turned on | `cargo xtask check rust` | Rust function length; no discarded results |

Each check is strict on every example a PR adds or changes, so a new example is checked as it is written. It only warns on untouched examples until the rewrites have cleared them; `--all-strict` makes every report an error. Biome can't be strict on changed files only, so its override warns everywhere and `check ts` lints the changed examples again with warnings as errors. The TypeScript and Rust pairs are listed in `tools/xtask/docs-snippets.toml`. The reviewer checklist covers what the tools can't judge.
