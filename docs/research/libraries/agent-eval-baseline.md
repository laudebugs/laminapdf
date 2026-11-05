# Agent-eval baseline: a coding agent on Lamina PDF tasks, with and without the shipped docs

**Not yet run.** The eval needs the owner's paid Anthropic API key, so no results exist yet. The task list, the checkers and the pinned agent are in place; the tables below list the tasks with "—" until the first run fills them in. To run it, from the repository root:

```sh
cargo xtask publish npm-pack
export ANTHROPIC_API_KEY=…
node tools/agent-eval/run.mjs
```

Then copy the tables from `target/agent-eval/<date>/results.md` into this page, as [the eval's README](../../../tools/agent-eval/README.md#record-a-baseline) describes.

## What the baseline measures

A new library is an API that every model has seen rarely or never, so agents guess its names and get them wrong ([report: a new library is a rare API to every model](shareable-libraries.md#a-new-library-is-a-rare-api-to-every-model)). Lamina PDF answers with version-matched docs inside `@laminapdf/sdk` (`node_modules/@laminapdf/sdk/docs/`) and an `AGENTS.md` snippet that points agents to them. This baseline records how often a pinned coding agent finishes twelve fixed tasks in two modes: `no-docs`, with the package's `docs/` folder left out and no `AGENTS.md`, and `docs`, with both. Later changes to names and docs are measured against it. A checker judges each result with Lamina PDF itself and `qpdf --check`; the [eval's README](../../../tools/agent-eval/README.md) describes the run.

## Run

| Field | Value |
| --- | --- |
| Date | — |
| Agent | claude-code 2.1.316 (`@anthropic-ai/claude-code`) |
| Model | `claude-sonnet-5-5` |
| Lamina PDF | — (the version of the packed `@laminapdf/sdk`) |
| Limits | 40 turns, 2 US dollars and 15 minutes per session; web search and web fetch off |
| Node, qpdf | — |

## Results by task and mode

| Task | What the agent must do | no-docs | docs |
| --- | --- | --- | --- |
| `damaged-file-warnings` | Open a file with broken cross-reference data, list its warning codes, save a repaired copy | — | — |
| `edit-metadata-incremental` | Set the title and author of an existing file and save incrementally | — | — |
| `encrypted-password` | Open an encrypted file and report `ok`, `wrong-password` or `password-required` from the error code | — | — |
| `extract-text` | Write the text of page 1 to a file | — | — |
| `image-with-caption` | Place a JPEG at a top-left position, embedded as it is, with a caption below | — | — |
| `links-and-outline` | Build three pages with an outline, a web link and a link to page 3 | — | — |
| `page-sizes-json` | List each page's displayed size in points as JSON | — | — |
| `rotate-page-3` | Rotate page 3 by 90 degrees and save incrementally | — | — |
| `set-metadata` | Build a page with title, author, subject and keywords | — | — |
| `tagged-pdf-ua` | Build a tagged PDF/UA-1 document with a heading and a paragraph | — | — |
| `text-at-position` | Draw text with its baseline at a given top-left position | — | — |
| `two-page-invoice` | Build a two-page invoice with a table, one row per line | — | — |
| **Passed** | | — / 12 | — / 12 |

## Details

The first run's per-run rows go here: the checker's reason for each result, the agent's time, its turns and its cost, copied from `results.md`.
