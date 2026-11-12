# Plan: docs readers can trust from the first page to the last (Round Q4)

- **Status:** Draft
- **Date:** 2026-11-11
- **Owner:** the docs track
- **Builds on:** [Quality](quality.md) (Round Q: docs as a story), [Docs examples](docs-examples.md) (Result tabs, Edit and run), [Docs diagrams](docs-diagrams.md), [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md) (no third-party loads, the CSP), the [discoverability plan](discoverability.md) (W3h's titles)
- **Research:** [What developers want from a docs site](../research/docs-site/developer-docs.md)
- **Todos:** Round Q4 on the [todo board](../todos/README.md); §4 lists them.

## 1. Goal

The docs research found that developers judge docs first on whether they are complete and correct, and then on whether they start from working code. Polish and interactivity come well behind. Lamina PDF's docs already get the hard part right: every TypeScript and Rust block comes from a tested example. Round Q4 makes that promise hold for every code block and puts each result where readers look. It also gives the reading half of the library the guides it lacks. It keeps Starlight and the site's look, and it changes the top-level structure only after watching readers use it.

## 2. What the research found

The [report](../research/docs-site/developer-docs.md) and its notes hold the evidence and sources. In short:

- **Wrong or missing docs drive readers away.** Studies at Microsoft and IBM rank missing, ambiguous and wrong content as the problems that make developers drop an API. On Hacker News, missing or outdated docs drew 431 comments in 116 of 197 threads about docs.
- **Examples are where errors pile up, and where readers start.** Code examples are 50% of the correctness issues in a study of 878 documentation issues. In the one eye-tracking study, all eleven developers started from example code.
- **The docs sites developers praise share working code, explanation and reachable versions.** Stripe leads 215 hand-coded "best docs" answers with 16, then Django (9) and PostgreSQL (8). Only 8 answers mention interactivity.
- **This site's gaps are in coverage and placement, and the framework isn't one of them.** The command-line transcripts are pasted by hand, and nothing checks them. Reading and extraction have one recipe and no guide. The page pictures sit behind a tab. There is only a Node first-run tutorial. Two recipes are missing from the sidebar.
- **Starlight stays.** Its own docs site loads about 6 KB of script, against 385–710 KB for the React-based options. Hosted platforms break ADR 0011, and switching fixes none of the gaps.

The command line already prints each error's docs page (`= see: <docs URL>`), so Q4h covers troubleshooting alone.

## 3. Order

The work goes in four steps, so the cheap, well-evidenced fixes ship first and the structural change waits for evidence from readers.

1. **Make the promise true, and close the gaps:** Q4a (command-line output from real runs), Q4b and Q4c (the reading path), Q4d (results beside the code), Q4e (first-run tutorials), Q4f (the sidebar), Q4h (troubleshooting), Q4i (the CSP), Q4j (the upgrade policy), Q4l (Markdown for agents).
2. **Measure:** Q4m watches developers follow the quick starts and the reading path, and reads the agent eval's results (L5b).
3. **Change the structure, if the measurements call for it:** Q4g groups the sidebar by task, with W3h's question titles.
4. **Add when there's time:** Q4k, Q4n and Q4o add one-page and PDF versions, version markers and usage pages for the two main API types. They depend on nothing else and rest on weaker evidence, so they can go in any order. Q4p, a spike, measured Twoslash hover types and recommends keeping them out.

## 4. Todos

| Todo | What it does | Recommendation |
| --- | --- | --- |
| [Q4a](../todos/done/Q4a-cli-output-from-real-runs.md) | Command-line transcripts and shell blocks generated from real runs, checked for drift | 1 |
| [Q4b](../todos/ready/Q4b-extraction-guide.md) | An extraction guide in TypeScript and Rust, and extraction on the home page | 2 |
| [Q4c](../todos/ongoing/Q4c-viewer-render-and-redaction-guides.md) | Guides for the viewer, rendering pages, and reveal and redaction from code | 2 |
| [Q4d](../todos/ready/Q4d-result-beside-the-code.md) | The first page picture beside the code; "Your first PDF" runnable | 3 |
| [Q4e](../todos/ready/Q4e-rust-and-cli-first-runs.md) | First-run tutorials for Rust and the command line | 4 |
| [Q4f](../todos/done/Q4f-every-page-in-the-sidebar.md) | The two recipes back in the sidebar, and a check that keeps every page there | 5 |
| [Q4g](../todos/ready/Q4g-sidebar-by-task.md) | The sidebar grouped by task, with question titles | 5 |
| [Q4h](../todos/ready/Q4h-troubleshooting-by-area.md) | Troubleshooting for Rust, the command line, extraction and rendering | 6 |
| [Q4i](../todos/ready/Q4i-code-colours-by-class.md) | Code colours by class, and no inline styles in the CSP | 7 |
| [Q4j](../todos/ready/Q4j-starlight-upgrade-policy.md) | Starlight upgrades one minor at a time, overrides listed | 8 |
| [Q4k](../todos/ready/Q4k-one-page-and-pdf-docs.md) | Each area on one page, and the whole guide as a PDF | 9 |
| [Q4l](../todos/ready/Q4l-markdown-for-agents-per-page.md) | A Markdown alternate link and "Copy as Markdown" on every page | 10 |
| [Q4m](../todos/ready/Q4m-watch-readers-use-the-docs.md) | Watch developers follow the docs, and read the agent eval | 13 |
| [Q4n](../todos/ready/Q4n-since-markers-and-versions.md) | "Added in" markers on API items, and stable URLs per release from 1.0 | 11 |
| [Q4o](../todos/ready/Q4o-usage-pages-for-document-and-builder.md) | Usage pages for `Document` and `Builder` beside their reference | 12 |
| [Q4p](../todos/done/Q4p-twoslash-spike.md) | A spike: Twoslash hover types on one page, measured against Q4i's CSP | — |

The recommendation numbers are the report's. Two of them are too large for one pull request, so the reading path is Q4b and Q4c, and the sidebar is Q4f now and Q4g after Q4m.

## 5. Out of scope

- **A move off Starlight**, a hosted docs platform, or a chat assistant on the site. The research found each costs more than it gives, and the hosted options break ADR 0011.
- **Twoslash hover types.** The [Q4p spike](../../spikes/docs-twoslash/README.md) measured them on one page. The plugin supports only older Expressive Code and TypeScript versions than Starlight ships. It adds 24.5 KB of script and 15.5 KB of CSS to every page, and on the page with hovers, 953 `style=` attributes that Q4i's CSP would block. Its hovers open by mouse but never by keyboard, and screen readers don't get them. It adds no type safety either: the docs show only example files, which CI already type-checks.
- **Personalised samples** in Stripe's style, which have nothing to personalise in a library without accounts.
- **Analytics.** ADR 0011 rules them out; Q4m measures by watching readers instead.

## 6. Risks

- **The research's weakest numbers are about interactivity and AI**, which come from vendors or keyword counts. Q4d and Q4l are small enough that being wrong costs little.
- **Regrouping the sidebar breaks links and habits.** Q4g waits for Q4m, keeps redirects for every moved path, and regenerates llms.txt in the new order.
- **Generated command-line output can change with every release.** Q4a fixes the inputs and dates, as the example goldens do, so a change in output is a real change.

## 7. To-dos

- [x] Q4a: command-line output from real runs
- [ ] Q4b: the extraction guide
- [ ] Q4c: viewer, rendering, and reveal and redaction guides
- [ ] Q4d: the result beside the code
- [ ] Q4e: first runs in Rust and on the command line
- [x] Q4f: every page in the sidebar
- [ ] Q4g: the sidebar by task
- [ ] Q4h: troubleshooting by area
- [ ] Q4i: code colours by class
- [ ] Q4j: the Starlight upgrade policy
- [ ] Q4k: one-page and PDF docs
- [ ] Q4l: Markdown for agents on every page
- [ ] Q4m: watch readers use the docs
- [ ] Q4n: "added in" markers and versions
- [ ] Q4o: usage pages for `Document` and `Builder`
- [x] Q4p: the Twoslash spike
