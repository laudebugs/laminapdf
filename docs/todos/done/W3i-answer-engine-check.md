# W3i — The answer-engine check

| Field | Value |
| --- | --- |
| ID | `W3i` |
| Status | `done` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | None |
| ADRs | None |
| Branch | `feat/w3i-answer-engine-check` |
| PR title | `feat(tools): ask answer engines a fixed set of questions and record what they say` |
| Size | M (a tool beside the agent eval, its questions, a first report) |
| PR | [#437](https://github.com/laudebugs/laminapdf/pull/437) |

## Goal

Round W3 changes what engines can read, but nothing tells the project whether answer engines recommend Lamina PDF more often as a result. Search Console and Bing report clicks and citations only for queries that already show the pages. This todo builds a small tool that asks a fixed set of developer questions ("What PDF library should I use to generate PDFs in TypeScript?", "How do I extract text from a PDF for RAG in Rust?", "Is there a PDF reader that doesn't upload my file?") to answer engines with web search, records whether Lamina PDF is mentioned or cited and which sources each answer cites, and runs a baseline before the round's bigger changes ship.

## Context

- [Discoverability research §4](../../research/discoverability/search-and-answer-engines.md#4-what-raises-the-chance-of-being-cited): which sources answers cite for developer tools.
- [`tools/agent-eval/`](../../../tools/agent-eval/README.md): the model for a tool run by hand with the owner's API key, never in CI, with pinned settings and results under `target/`.

## Scope

**In scope**

- `tools/answer-check/`: a question list (about 20, in plain wording, none naming Lamina PDF), a runner that asks each one through an API that answers with web search and returns its sources (start with the Claude API's web search tool; add others the owner has keys for), and a report: per question, mentioned or not, cited URLs, Lamina PDF's URLs among them.
- Each answer asked three times, since answers vary; the report gives the count.
- The baseline report committed as a research note in `docs/research/discoverability/`.

**Out of scope**

- Running in CI or gating on it: answers aren't deterministic and the API costs money.
- Scraping consumer chat products: only documented APIs.

## Tasks

- [x] Read the current API docs for the search tool and pin the model and settings in a config file, as the agent eval does.
- [x] Questions file, runner, report (JSON and a Markdown table); `--dry-run` prints the requests.
- [x] Tests of the report from recorded answers (no network).
- [x] The owner runs the baseline; commit the note with the date and settings.

## Acceptance criteria

- [x] The tool runs with only an API key in the environment and writes its report under `target/answer-check/`.
- [x] A committed baseline note.

## As built

- **Two engines.** The tool asks Claude (the Messages API's web search tool) or OpenAI (the Responses API's `web_search` tool) and reads both answers into one report. The baseline uses OpenAI, because the session that ran it could reach OpenAI's API with the owner's key but not Anthropic's. Later runs compare with it only if they ask OpenAI with the same settings, so `config.json`'s default engine is OpenAI. The model is `gpt-6.1-sol`: a full run of 60 answers costs about $4, against about $13 with `gpt-6-astra`, whose first attempts ran out of the account's credit.
- **Background mode for OpenAI.** The first run stopped on a question whose answer took over 30 seconds, where the session's proxy cut the request off. The tool now starts each OpenAI answer in background mode and reads it every three seconds, and retries a read that fails for a passing reason (the network, 408, 429, 5xx).
- **Sources on every read.** A background response read back without `include[]=web_search_call.action.sources` has no search results, which made the first complete run's "in results" column meaningless; the code review caught it and the baseline was run again. The POST that starts an answer is sent once, since a retry could start a second, billed answer; an answer the run gives up on is cancelled.
- **One folder per engine and run:** `target/answer-check/<date>-<engine>/`, so runs of the two engines on one day never overwrite each other.

## Post-merge checks

- [ ] A second run after W3d and W3e ship, compared with the baseline in the note.

## Tests

Unit tests of the report over recorded answers.

## Docs to update

`tools/answer-check/README.md`, CLAUDE.md's tools list and commands, `docs/research/discoverability/README.md`, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- A tool that sends the questions to an API sends nothing about the user or the project but the questions; keep it that way.
- Answer engines change week to week; compare runs with the same settings and report spread, not single answers.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
