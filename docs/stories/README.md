# Stories

Short posts that tell Lamina PDF's story one part at a time, for LinkedIn, X and Threads. Each tells one moment from the project: a bug and how it was proven fixed, a decision and what it cost, a measurement and why it matters, how the work gets done with AI agents. The aim is twofold: to bring people to Lamina PDF, and to share what building it this way teaches.

The [`lamina-story` skill](../../.agents/skills/lamina-story/SKILL.md) writes them. Ask an agent for "a story about …", or for "the next story". Each story is a LinkedIn post of 150–200 words, a post for X and one for Threads, and at least one image per platform in the Paper look. Every fact is checked against the repository before the owner sees it, and only the owner posts.

## How a story is kept

Each story has a folder, `<NNN>-<slug>/`: `story.md` (the posts, the alt text, the counts, the fact table with its evidence, and the checks) and `visuals/` (each image's HTML and its rendered PNG). Every post is written with `/writing`, reviewed with `/writing-review` and fact-checked against the repository before the owner sees it. The images are drawn from HTML in the site's tokens, fonts and one card style sheet (`apps/site/scripts/story-card.css`):

```bash
pnpm --filter @laminapdf/site story-image docs/stories/<NNN>-<slug>
```

## The coverage map

One row per story worth telling. **Status** is `open` (not drafted), `drafted` with a link to its story (`drafted: [001](001-slug/story.md)`, waiting for the owner), or `told` with the date and the link (posted). Pick from `open` rows; a `told` row gets a second story only with a new angle, written in the row. Add a row whenever the project makes a new story: a bug fixed, a decision recorded, a measurement published.

### The product

| Topic | The story in a line | Sources | Status |
| --- | --- | --- | --- |
| Small enough for the web | Typst would have added 7.4 MB to the download; a layout engine of our own added 19 KB | [flow-layout spike](../../spikes/flow-layout/README.md), [ADR 0032](../decisions/0032-flowing-layout-and-document-inputs.md) | open |
| How fast it is | What the benchmarks measure, on which machines, and what makes it fast | [benchmarks data](../../apps/docs/src/data/benchmarks.json), [size budgets](../../packages/wasm/size-budgets.json) | open |
| Your PDF never leaves your device | The reader opens documents without sending them to any server, and ten guarantees say how to check | [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md) | open |
| What your PDF is hiding | Text under black boxes, earlier revisions, metadata: `laminapdf reveal` shows what a file carries | [trust and proof plan](../plans/trust-and-proof.md), [`reveal.rs`](../../crates/laminapdf/src/reveal.rs) | open |
| Real PDFs are broken | Readers must tolerate damaged files, while writers stay strict | [ADR 0003](../decisions/0003-reader-tolerance-and-limits.md) | open |
| PDFs that AI can read | Reading order, tables and hidden text for retrieval, measured on a public benchmark | [ADR 0030](../decisions/0030-extraction-output-and-model-plugins.md), [extraction README](../../crates/laminapdf-extract/README.md) | open |
| One core, every platform | One Rust core behind the web, a CLI, an MCP server and mobile bindings | [ADR 0001](../decisions/0001-core-language-and-licence.md), [ADR 0031](../decisions/0031-cli-and-mcp-contract.md), [mobile spike](../../spikes/lamina-ffi/README.md) | open |
| Two renderers, one truth | PDFium draws the pages; hayro checks it in CI | [ADR 0004](../decisions/0004-render-backend.md) | open |
| A calm reader | Paper, a design language that gets out of the page's way | [design language](../design/paper/design-language.md) | open |
| Errors that explain themselves | Every error has a code, a hint and a page of its own | [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) | open |

### How it's built

| Topic | The story in a line | Sources | Status |
| --- | --- | --- | --- |
| Why Lamina PDF exists | Where the project started, in the owner's words | the owner | open |
| Blurry at some zooms, sharp at others | Two parts of the viewer sized a page one pixel apart, and a sweep of 14,464 cases proved the fix | [the bug](../bugs/viewer-blurry-at-zoom/detail.md), [its resolution](../bugs/viewer-blurry-at-zoom/resolution.md) | open |
| How we fix a bug | Reproduce it as a user would, measure every stage, fix the cause, prove it with a sweep, write it down | [bug triage](../guidelines/bug-triage.md), [bugs](../bugs/README.md) | open |
| The accent on the wrong letter | Three Arabic diacritics landed on the neighbouring letter, while pdf.js read them right | [the bug](../bugs/rtl-mark-on-wrong-letter/detail.md) | open |
| Checked against other readers every night | Lamina PDF's answers compared with pdf.js, PDFium, qpdf and pypdf, plus fuzzing | [test strategy](../decisions/0018-test-strategy.md), [fuzz README](../../fuzz/README.md), [`proof.rs`](../../tools/xtask/src/proof.rs) | open |
| Listening before building | What people on Hacker News say they lack in PDF tools, and the plan it changed | [the research](../research/hacker-news/pdf-users-on-hacker-news.md), [trust and proof plan](../plans/trust-and-proof.md) | open |
| AI agents on a todo board | One todo per pull request, a review before every merge, many agent sessions in parallel | [todo board](../todos/README.md), [git workflow](../guidelines/git-workflow.md) | open |
| Decisions written down | Every architecture choice is an ADR, so agents and people work from the same record | [ADRs](../decisions/README.md) | open |
| A release every day | A bot cuts a release after the nightly passes | [release README](../../tools/release/README.md) | open |

## Told

Stories in the order they were drafted, with where they were posted.

| # | Story | Topic | Status | Posted |
| --- | --- | --- | --- | --- |
| — | None yet | | | |
