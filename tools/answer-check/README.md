# Answer-engine check

This check measures whether an AI answer engine recommends Lamina PDF when a developer asks it for a PDF library or reader. It asks twenty fixed questions, none of which names Lamina PDF, to Claude with web search. Each question is asked three times, because answers vary. For every answer it records three things: whether the text names Lamina PDF, whether one of Lamina PDF's pages came back in the searches, and whether the answer cites one. The [discoverability plan](../../docs/plans/discoverability.md) uses it to tell whether its work changes what answer engines say: a baseline before the bigger changes ship, then a run after each step. It is run by hand and never gates anything, because it needs a paid API key and answers change from day to day.

## Run it

You need Node 22 or later and Anthropic API credentials.

```sh
pnpm --dir tools/answer-check install --frozen-lockfile   # its own pnpm project, outside the workspace
export ANTHROPIC_API_KEY=…                                # or sign in once with `ant auth login`
node tools/answer-check/run.mjs --dry-run                 # the requests it would send, without sending any
node tools/answer-check/run.mjs                           # every question, three times
node tools/answer-check/run.mjs --questions rag-ts,pdf-mcp --repeats 1
```

The credentials are read from the environment, or from an `ant auth login` profile, never from a file in the repository. Requests go one at a time, so a full run of 60 answers takes several minutes.

## What it asks, and how

`questions.json` holds the questions, each with an `id` and an area: the library, extraction, tools and the reader. They are written as a developer would type them. A test checks that none names Lamina PDF, so an answer only mentions it if the engine found it.

`config.json` pins the settings that make two runs comparable:

- the model (`claude-opus-5-5`) and its effort (`medium`);
- the web search tool (`web_search_20260209`) and at most five searches per answer;
- three runs per question;
- what counts as Lamina PDF: its names (`Lamina PDF`, `laminapdf`, `@laminapdf/`), its domain and subdomains, and its pages on GitHub, npm, crates.io, lib.rs and docs.rs.

Each request holds the question alone, with no system prompt. A turn the server pauses after many searches is resumed, at most three times. Requests opt into the API's fallback model: if the model declines a question, another model answers it instead, and the answer records which model served it.

## Results

Each run writes to `target/answer-check/<date>/`, or `<date>-2` and so on for a second run that day:

- `answers.json`: the settings, then every answer with its text, search queries, result URLs, cited URLs, the model that served it, its stop reason and its token and search counts;
- `report.md`: the totals, then a row per question with how many runs named Lamina PDF, found its pages, and cited them, and the hosts that answer cited most.

## Cost

Search costs at most 3 US dollars for a full run: 60 answers, each with up to five searches at 10 dollars per 1,000. The tokens cost more, since the search results go into each request. `report.md` gives the input and output token and search totals, so the cost of a run can be worked out from the current prices. While you work on the questions or the report, run one question once with `--questions <id> --repeats 1`.

## Record a run

The first full run is the baseline. To record it, or a later run:

1. run every question on a clean checkout of `main`;
2. copy the totals and the table from `report.md` into the [baseline note](../../docs/research/discoverability/answer-engine-baseline.md), with the date and settings;
3. open a docs PR with the note.

Keep `questions.json` and `config.json` unchanged between runs you want to compare. A change to either starts a new baseline; say so in the note.

## Change the check

`report.mjs` reads an answer's content blocks and writes the report; `run.mjs` sends the requests. `node --test tools/answer-check/test/*.test.mjs` checks both on recorded answers, without the network. To add a question, add it to `questions.json` with a new `id`.
