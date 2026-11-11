# Q4a — Command-line output in the docs from real runs

| Field | Value |
| --- | --- |
| ID | `Q4a` |
| Status | `ready` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0031](../../decisions/0031-cli-and-mcp-contract.md) (the command line's output and exit codes) |
| Branch | `docs/q4a-cli-transcripts` |
| PR title | `docs(cli): command-line output generated from real runs and checked for drift` |
| Size | M |
| PR | — |

## Goal

The docs promise that every code block was tested, and for TypeScript and Rust it holds. The command-line pages break it: their 16 `console` transcripts were pasted by hand, and no check runs them. Wrong examples are the complaint the research found most damaging, so this todo makes every transcript and shell block on the docs site the output of a real run, written by a test and checked for drift like the example goldens.

## Context

- [Docs research: recommendation 1](../../research/docs-site/developer-docs.md#recommendations-in-order), and the [audit](../../research/docs-site/developer-docs/current_docs_site.md) that found the hand-pasted transcripts.
- `apps/docs/src/content/docs/cli/*.mdx`: the 16 `console` blocks (inspect 5, build-and-edit 4, scripts 3, extract-and-render 2, index 1, install 1), and the `sh` blocks on the install and quick-start pages.
- `crates/laminapdf-cli/tests/` and `fixtures/golden/cli-rotate-hello.pdf`: how the CLI's tests already pin its output.
- `apps/docs/src/components/Snippet.astro`: how a page includes a tested file today.

## Scope

**In scope**

- A transcript format: the command, its fixed inputs (fixtures, dates, `--json` where the page shows it) and its expected stdout, stderr and exit code, one file per transcript under `fixtures/cli/transcripts/`.
- A test in `crates/laminapdf-cli/tests/` that runs each transcript's command and compares; `UPDATE_GOLDEN=1` rewrites them.
- A docs component that shows a transcript as a `console` block, and the CLI pages moved onto it.
- The `sh` install blocks: checked against the version and package names they name (the install commands can't run in a test).

**Out of scope**

- The MCP page's JSON examples, unless they come out of the same run cheaply.
- New command-line pages (Q4e adds the first-run tutorial).

## Tasks

- [ ] List every `console` and `sh` block on the site, and what each needs to run.
- [ ] Write the transcript format and the test; make the outputs deterministic (fixed fixtures, no timings or paths that vary).
- [ ] Add the component (`Transcript.astro` or an option on `Snippet`), and make `check docs-snippets` refuse a hand-written `console` block.
- [ ] Move each CLI page onto the transcripts, fixing any output that has drifted.
- [ ] Check the `sh` install blocks against the package names and the version rule on the install page.

## Acceptance criteria

- [ ] Every `console` block on the site comes from a transcript file that a test runs.
- [ ] A change to a command's output fails the test until the transcript is updated.
- [ ] `check docs-snippets` fails on a hand-written `console` block.

## Post-merge checks

None.

## Tests

The new transcript test in `crates/laminapdf-cli/tests/`; the docs build; `check docs-snippets`.

## Docs to update

The CLI pages; `docs/guidelines/examples.md` (transcripts are examples too); `fixtures/README.md`.

## Risks and open questions

- Some output names the version or a file size; fix the inputs, or mask those fields in the comparison, rather than leaving them hand-written.
- Terminal-only output (colours, a progress line) differs from piped output; the transcripts show what a reader sees in a terminal, so decide how the test renders it.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
