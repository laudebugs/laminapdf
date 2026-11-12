# Q4e — First-run tutorials for Rust and the command line

| Field | Value |
| --- | --- |
| ID | `Q4e` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4a](./Q4a-cli-output-from-real-runs.md) |
| ADRs | None |
| Branch | `docs/q4e-rust-cli-first-runs` |
| PR title | `docs: first PDFs in Rust and on the command line` |
| Size | M |
| PR | [laudebugs/lamina#445](https://github.com/laudebugs/laminapdf/pull/445) |

## Goal

The only first-run tutorial is for Node. A Rust developer lands on a quick start that links to docs.rs, and a command-line user on an install page. This todo writes a first-run tutorial for each, as "Your first PDF" is for Node: what to install, the version to pin and why, one command or program, and the PDF it makes.

## Context

- [Docs research: recommendation 4](../../research/docs-site/developer-docs.md#recommendations-in-order); a Hacker News commenter's "zeroth step missing" quoted there.
- `apps/docs/src/content/docs/start/first-pdf.mdx`: the Node tutorial to match.
- `quick-start/rust.mdx`, `cli/install.mdx`: what exists.
- Q4a's transcripts, which the command-line tutorial's output comes from.

## Scope

**In scope**

- `start/first-pdf-rust`: `cargo new`, the dependency with its version pin explained, `hello.rs`, the result.
- `start/first-pdf-cli`: install, then `info`, `check` and `extract` on a sample file, and `build` on a Markdown file, with transcripts from Q4a.
- Both in "Start here", linked from the home page's language choice.

**Out of scope**

- Other runtimes' tutorials (the quick starts cover them).

## Tasks

- [x] Write the Rust tutorial from `crates/laminapdf/examples/hello.rs`, checked by the examples' tests.
- [x] Write the command-line tutorial on Q4a's transcripts.
- [x] Link both from the home page and the install page.

## Acceptance criteria

- [x] Each tutorial ends with a PDF the reader made, shown on the page. *As done:* the Rust one with its page picture; the command-line one with its text read back and `render` writing a PNG, but no picture, since a committed PNG from `render` can differ across CPU architectures.
- [x] Every code block and transcript in them is tested: the Rust program is the tested `hello` example (and the steps were run against the published crate), every session a transcript; the install `sh` lines are the install page's.

## Post-merge checks

None.

## Tests

The examples' tests; Q4a's transcript test; the docs build.

## Docs to update

"Start here", the home page, the install page.

## Risks and open questions

- The version pin changes with every release; the tutorials must read it from the same source as the install page.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
