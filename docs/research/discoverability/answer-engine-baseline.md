# Answer-engine baseline: does Claude with web search recommend Lamina PDF?

- **Status:** Not yet run
- **Asked for by:** the [discoverability plan](../../plans/discoverability.md) (todo W3i)
- **Tool:** [`tools/answer-check/`](../../../tools/answer-check/README.md)

This note records what an AI answer engine says when a developer asks it for a PDF library or reader, before Round W3's bigger changes ship. It will hold the first full run of the answer-engine check: twenty questions that don't name Lamina PDF, each asked three times to Claude with web search. Later runs, after W3d and W3e ship, are compared with it here.

## Settings

The check's `config.json` and `questions.json` as committed with this note: model `claude-opus-5-5` at effort `medium`, the `web_search_20260209` tool with at most five searches per answer, three runs per question, the question alone with no system prompt.

## Results

Not yet run: the run needs an Anthropic API key. Once run, this section holds the date, the token and search totals, and the table from `report.md`.
