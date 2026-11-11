# What Hacker News says about developer documentation (2015 – November 2026)

Data: 197 Hacker News stories about developer documentation with at least 50 comments each, posted 16 March 2015 to 11 October 2026, and all 21,499 comments under them (comment dates 16 March 2015 to 16 October 2026), plus 10 adjacent threads on error messages and changelogs (966 comments) counted apart. Collected 11 November 2026 through the HN Search API ([search](https://hn.algolia.com/api/v1/search), [items](https://hn.algolia.com/api/v1/items/41439983)). Scripts and data: `tools/hn-research/developer-docs/` (`collect_stories.py`, `select.py`, `collect_comments.py`, `themes.py`, `strict.py`, `story_groups.py`, `nominations.py`, `grep.py`, `verify.py`; outputs in `hn_work/data/`). Every quote below was checked word for word against the comment's text fetched again from the items API by `verify.py`.

## How the data was collected and how far the counts can be trusted

### Takeaway
The counts are reproducible but keyword rules are blunt: spot checks put their precision between about 25 % and 90 % depending on the theme, so treat theme counts as rough sizes and the hand-coded counts (the "best docs" nominations, the thread groups) as the firmer evidence.

### Cited Findings
- Stories: 58 title queries (from "documentation", "docs", "API documentation", "Diátaxis", "README", "llms.txt", "Docusaurus", "Mintlify", "man pages", "playground", "AGENTS.md" to "Arch wiki"), one year at a time, stories only, typo tolerance off, ≥ 50 comments: 865 distinct hits. Every title was read; 197 were kept as about developer or software documentation (docs sites, docs tooling, man pages, READMEs, writing docs, docs for LLMs and agents) and 10 as adjacent (error messages, changelogs). Dropped on reading: Google Docs the product, leaked or court "docs", company-internal design docs and meeting culture, self-documenting-code debates, technical books, tutorials themselves — [select.py](../../../../tools/hn-research/developer-docs/select.py), [HN Search API](https://hn.algolia.com/api/v1/search?query=documentation&tags=story)
- Split by comment date: 11,954 comments in 2015–2022 and 9,545 in 2023–2026; by story date 107 threads before 2023 and 90 from 2023 on — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- Thread groups by title (hand rules, first match wins; share of the 21,499 comments): writing and structuring docs 49 threads, 22.5 %; single projects and platforms 33 threads, 16.6 %; AI, LLMs, agents and llms.txt 29 threads (all 2023 or later), 16.1 %; MDN 22 threads, 14.8 %; docs tooling and formats 33 threads, 12.9 %; man pages, tldr and info 11 threads, 5.9 %; Apple developer docs 6 threads, 5.9 %; "best/worst docs" Ask HN threads and rants 14 threads, 5.3 % — [story_groups.txt](../../../../tools/hn-research/developer-docs/data/story_groups.txt)
- Counting rule: a comment counts for a theme when its text matches the theme's regular expression; denominators are all comments in the 197 core threads (or the period's comments). `themes.py` matches anywhere in the comment; `strict.py` requires a documentation word and the theme's cue in the same sentence. Both print comments, distinct threads, and per-period counts and shares — [themes.py](../../../../tools/hn-research/developer-docs/themes.py), [strict.py](../../../../tools/hn-research/developer-docs/strict.py)
- Precision from reading 8 random matches per rule (seeded, reproducible with `--samples THEME 8 SEED`): missing/outdated 7/8; offline/plain/single-page 7/8; examples (strict) 6/8; AI distrust/hallucination 6/8; AI-generated docs 5/8; docs for agents/llms.txt topical on 8/8 (stance mixed); reference-only/needs guides ~4/8; findability/search ~3–4/8; marketing 4/8 (`themes.py`), 2/8 (`strict.py`); interactive (strict) 3/8; video (strict) ~4/8; scattered (strict) 2/8; "AI instead of docs" 1/8; slow/JS-heavy 0–2/8; versioning 0–1/8 — [themes.py](../../../../tools/hn-research/developer-docs/themes.py), [strict.py](../../../../tools/hn-research/developer-docs/strict.py)
- What keyword rules cannot tell apart: praise from complaint ("great search" and "terrible search" both match); a commenter's own view from a quoted or reported one; "example" meaning a code example from "for example" (stripped before matching, but "an example of good docs" still counts); docs sites from internal wikis; a thread's topic from a comment's theme (the Discord thread inflates "scattered", the React SPA thread inflated "slow/JS-heavy" until excluded); sarcasm. Sample sizes of 8 give only coarse precision.

### Inferences
- For a redesign, the strongest numbers here are the hand-coded nominations (215 top-level comments) and the thread-level facts (which stories drew hundreds of comments); theme counts show relative size and change over time, not exact prevalence.

### Gaps
- Comment scores are not in the items API, so "most upvoted" views could not be counted.
- Threads under 50 comments (many "Stripe docs" or "Show HN: docs tool" posts) are outside the sample.

## What developers complain about in docs

### Takeaway
The biggest, most reliably measured complaints are docs that are missing, wrong or out of date (431 comments in 116 of 197 threads) and docs that list functions without explaining how to use them; examples are wanted everywhere, and "wrong docs are worse than no docs" recurs across the decade.

### Cited Findings
- Missing or outdated docs: 431 comments (2.0 %), 116 threads; 2.1 % of 2015–2022 comments, 1.9 % of 2023–2026; precision about 7/8 — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- Examples (strict rule, a docs word and "examples/sample code/snippet" in one sentence): 481 comments in 115 threads, 2.3 % then 2.2 %; precision about 6/8. Comments that examples don't work, are broken, or should be tested: 39 comments in 30 threads (strict) — [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt)
- Reference-only docs, no guides, no "why": 170 comments in 75 threads (`themes.py`), precision about half — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- Marketing in docs and landing pages that don't say what the thing does: 328 comments match the loose rule (about half on theme) and 105 the strict one — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt), [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt)
- Search and navigation: 412 comments in 139 threads (loose rule, roughly half on theme) — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- Assumed knowledge and tone ("just", "simply", jargon, beginners): 644 comments in 166 threads (loose, ~3/8 on theme); a whole 2023 story, "Just Simply – Stop saying how simple things are in our docs", drew 288 comments — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt), [story 35759449](https://news.ycombinator.com/item?id=35759449)
- Apple's developer docs are the most-discussed bad docs: six stories (2019–2026) with 1,258 comments, including "Apple Developer Documentation Is Missing" (389 comments) and "On Apple's Piss-Poor Documentation" (335) — [story_groups.txt](../../../../tools/hn-research/developer-docs/data/story_groups.txt), [21376744](https://news.ycombinator.com/item?id=21376744), [25046691](https://news.ycombinator.com/item?id=25046691)
- Named as worst in the "crappiest documentation" Ask HN (2017, 26 top-level comments): OpenSSL (2), Caffe (3), OpenCV (2), AWS, TensorFlow tutorials, WebRTC, Socket.io, Microsoft Office JS — [14575014](https://news.ycombinator.com/item?id=14575014)

Quotes, missing and wrong docs:
- "Outdated documentation is worse than no documentation. It doesn't matter which framework was used to write it." — [26012542](https://news.ycombinator.com/item?id=26012542), 2021-02-03
- "Wrong documentation is even worse than no documentation." — [35300979](https://news.ycombinator.com/item?id=35300979), 2023-03-25
- "I've seen so much outdated documentation recently that I'm starting to ignore it and go straight to code" — [38907658](https://news.ycombinator.com/item?id=38907658), 2024-01-08
- "The astounding lack of documentation is almost amusing." — [21379048](https://news.ycombinator.com/item?id=21379048), 2019-10-28
- "Some personal favorites, culled by searching my source code for swear words" — [19967377](https://news.ycombinator.com/item?id=19967377), 2019-05-21

Quotes, examples and tested examples:
- "Given the choice between massive, "complete" documentation with no examples and decent but incomplete docs with good examples, I’ll pick the second every time." — [45537381](https://news.ycombinator.com/item?id=45537381), 2025-10-10
- "I'd say worked examples and comprehensive references are mutually reinforcing kinds of documentation, meaning you want both." — [45542456](https://news.ycombinator.com/item?id=45542456), 2025-10-10
- "It's so infuriating coming across a project that only has one kind and not the other." — [45544325](https://news.ycombinator.com/item?id=45544325), 2025-10-10
- "A simple example goes A LONG WAY." — [22120024](https://news.ycombinator.com/item?id=22120024), 2020-01-22
- "We need a way to unit test documentation to see if the code snippets still compile and the things that are referenced still exist." — [25423260](https://news.ycombinator.com/item?id=25423260), 2020-12-14
- "In rustdoc, documentation examples are compiled and run whenever you run your tests." — [15562648](https://news.ycombinator.com/item?id=15562648), 2017-10-26
- "This is why I prefer to link to actual code in the repo, in an examples directory, which gets compiled during every build." — [25423296](https://news.ycombinator.com/item?id=25423296), 2020-12-14

Quotes, reference without the "why", marketing, tone:
- "documenting the "what" but neglecting the "why" or "how"." — [21377412](https://news.ycombinator.com/item?id=21377412), 2019-10-28
- "It isn't sectioned just a dumb list of methods where similar methods can be pages away from each other" — [22691591](https://news.ycombinator.com/item?id=22691591), 2020-03-26
- "That's fine, but put your marketing copy in your marketing, not in your docs." — [35774293](https://news.ycombinator.com/item?id=35774293), 2023-05-01
- "Don't say your product is the best, the greatest, the simplest, the most powerful. That's marketing BS, noise." — [31963395](https://news.ycombinator.com/item?id=31963395), 2022-07-02
- "Most stuff starts on marketing fluff nowadays instead of the real meat" — [40570182](https://news.ycombinator.com/item?id=40570182), 2024-06-04
- "Just explain briefly what the project is about." — [15564574](https://news.ycombinator.com/item?id=15564574), 2017-10-27
- "Most documentation reads like you should already know what you’re doing, which makes sense because it was written by someone who already knows how to do the process." — [50042767](https://news.ycombinator.com/item?id=50042767), 2026-10-11

### Inferences
- The two most consistent demands are "correct and current" and "examples plus reference, not either alone"; the 2025 thread "Examples are the best documentation" (173 comments) ended mostly at "both", not examples only. Tested examples (rustdoc doctests, examples compiled in CI) come up as the accepted fix for examples that rot.
- Apple shows that a platform owner's docs can be a sustained, years-long reputational problem on HN, not a one-off.

### Gaps
- How many complaints are about open-source projects versus vendor APIs was not coded.

## Which docs they praise, and why

### Takeaway
Asked directly for the best documentation they have read, HN commenters name Stripe most (16 of 215 top-level answers), then Django (9), PostgreSQL (8) and the Emacs manuals (8); the reasons given are examples, completeness with depth, versions kept available, copy-paste code with your own credentials, and docs that link to source.

### Cited Findings
- Hand-coded nominations in five Ask HN threads (June 2018 to February 2023; 215 top-level comments; a comment can name several): old printed manuals as a group 18; Stripe 16; Django 9; PostgreSQL 8; Emacs/Elisp/Org 8; scientific Python (scikit-learn, NumPy, SciPy, pandas) 6; MATLAB 6; MDN 5; Rust 5; BSD docs and man pages 5; Qt 5; Python stdlib 4; Requests 4; SQLite 4; Vue, Redis, cppreference, Arch wiki, Vim, Mathematica, Diátaxis 3 each; Twilio, Tailwind, Laravel, Elasticsearch, Lua, ZeroMQ, FastAPI, Microsoft, Perl, Kubernetes, AWS, Slack, PHP, Racket, CL HyperSpec 2 each — [nominations.txt](../../../../tools/hn-research/developer-docs/data/nominations.txt), threads [17399340](https://news.ycombinator.com/item?id=17399340), [17905919](https://news.ycombinator.com/item?id=17905919), [32794330](https://news.ycombinator.com/item?id=32794330), [33682599](https://news.ycombinator.com/item?id=33682599), [34820382](https://news.ycombinator.com/item?id=34820382)
- Of those 215 answers, a regex finds 39 mentioning examples, 21 concepts/overview/"how it fits together", 11 search or findability, 8 interactivity or "try it", 8 offline/built-in docs, 8 links to source, 7 versions, 6 single-page docs (regex counts on the same 215; "example" includes "for example") — [nominations.txt](../../../../tools/hn-research/developer-docs/data/nominations.txt)
- Mentions across all 22,465 comments (any context; a sentence with the name and a positive or negative word, a crude polarity): MDN 891 comments (127 positive, 24 negative sentences, mostly inside the 22 MDN threads); Stripe 66 comments in 34 threads (19 positive, 1 negative); Django 74 in 29 threads (24 / 5); PostgreSQL 73 in 30 (18 / 2); Arch wiki 59 in 14 (13 / 2); OpenBSD 68 in 22 (12 / 4); SQLite 56 in 20; Rust book/docs 135 in 51 (17 / 6); docs.rs 5; Tailwind 18; Laravel 9; FreeBSD 40 in 21 — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- MDN is the docs site HN rallies around: 22 stories, 3,179 comments (14.8 % of the sample), from the 2017 "Documenting the Web together" to the 2020 layoffs ("Thank You MDN", 319 comments) and MDN Plus — [story_groups.txt](../../../../tools/hn-research/developer-docs/data/story_groups.txt), [24136895](https://news.ycombinator.com/item?id=24136895)
- A dedicated thread "Why is the OpenBSD documentation so good?" (91 comments) and "Arch Linux turns 20: Small, simple, great documentation" (149) show the BSD man pages and Arch wiki as standing examples — [32515928](https://news.ycombinator.com/item?id=32515928), [33729445](https://news.ycombinator.com/item?id=33729445)

Quotes, why these sites:
- "the code samples (in 6-8 languages) are copy-paste ready using your own pre-populated credentials." — [32794798](https://news.ycombinator.com/item?id=32794798), 2022-09-10 (Stripe)
- "Having interactive examples alongside the documentation (in multiple languages nonetheless) was something that stood out to me the first time I had to read the Stripe docs" — [34835320](https://news.ycombinator.com/item?id=34835320), 2023-02-17
- "Auth0 even inlines your credentials into the example code if you're logged in." — [17906429](https://news.ycombinator.com/item?id=17906429), 2018-09-04
- "Clearly noting what versions things changed. Ability to switch versions at the click of a button." — [33682987](https://news.ycombinator.com/item?id=33682987), 2022-11-20 (Django)
- "Django, because there are no missing parts. Everything is documented and comes with examples, that work" — [32795354](https://news.ycombinator.com/item?id=32795354), 2022-09-10
- "they do a great job of keeping docs for older versions available." — [34821707](https://news.ycombinator.com/item?id=34821707), 2023-02-16 (PostgreSQL)
- "you can click the "code symbol" (</>) to go to the actual implementation of the function documented." — [32794714](https://news.ycombinator.com/item?id=32794714), 2022-09-10 (Elixir/hexdocs)
- "To me MDN is the standard bearer for good language/library documentation. It has example uses." — [41526926](https://news.ycombinator.com/item?id=41526926), 2024-09-13

### Inferences
- The praised sites share concrete, copyable mechanics more than visual design: runnable or copy-ready code per language, versions you can switch to, docs that explain concepts as well as list APIs, and a route to the source. Stripe is the default answer for API docs; Django and PostgreSQL for library/product manuals that mix guide and reference.
- Nostalgia for printed manuals (18 nominations) reflects a wish for complete, coherent, single-author documents rather than a request for print.

### Gaps
- No "best docs" Ask HN thread with ≥ 50 comments after February 2023 was found, so nominations do not reflect 2024–2026 sites (Tailwind, Svelte, docs built with Starlight or Mintlify are barely mentioned).

## Form and site details: search, versions, offline, man pages, interactivity, video, UI

### Takeaway
HN readers value docs they can reach fast and in their own tools — one-page or offline copies, man pages, a working search, a version switcher — and dislike heavy JavaScript sites and information locked in videos or Discord; interactive examples are praised mainly on API sites, and UI niceties (dark mode, copy buttons) draw little discussion.

### Cited Findings
- Offline, plain-text, single-page, PDF or terminal access: 252 comments in 92 threads (strict, ~7/8 on theme), 1.2 % then 1.1 % — [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt)
- Man pages: 722 comments in 85 threads (any mention; mostly in 11 man-page/tldr threads, 1,276 comments); "TLDR pages" twice drew 162 and 167 comments, and "Man pages are great, man readers are the problem" 162 — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt), [15779382](https://news.ycombinator.com/item?id=15779382), [43631672](https://news.ycombinator.com/item?id=43631672)
- Scattered docs (Discord, Slack, GitHub issues, videos): the strict rule is too noisy to count (2/8), but one 2023 story, "Discord is not documentation", drew 289 comments — [36746154](https://news.ycombinator.com/item?id=36746154)
- Interactive examples and playgrounds: 96 comments (strict, ~3/8 on theme); 8 of 215 "best docs" answers cite interactivity — [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt), [nominations.py](../../../../tools/hn-research/developer-docs/nominations.py)
- Video versus text: 53 comments (strict, ~half on theme); the on-theme ones prefer text — [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt)
- Slow or JavaScript-heavy docs sites and version selectors could not be counted reliably (keyword precision ≤ 2/8); evidence below is quotes only — [strict.py](../../../../tools/hn-research/developer-docs/strict.py)
- UI details: "dark mode/theme" appears in 17 comments, both asked for and disliked; copy buttons almost never come up — [grep.py](../../../../tools/hn-research/developer-docs/grep.py)
- Error messages that link to docs: 8 comments in the 5 error-message threads (adjacent set, 966 comments in all 10 adjacent threads); changelogs and release notes: 183 comments in the 5 changelog threads, 49 in the core threads — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)

Quotes:
- "I often find myself clicking on "print" links on sites to make things show up on a single page so I can locate things quickly." — [11629915](https://news.ycombinator.com/item?id=11629915), 2016-05-04
- "Lua's manual. One flat page with everything." — [33684554](https://news.ycombinator.com/item?id=33684554), 2022-11-20
- "Node.js because they propose a single page version and it makes search easier" — [17406581](https://news.ycombinator.com/item?id=17406581), 2018-06-27
- "Is it possible to make a PDF or an offline version?" — [13664483](https://news.ycombinator.com/item?id=13664483), 2017-02-16 (the Rust book)
- "offline versions of most of these sorts of resources load considerably faster" — [30798520](https://news.ycombinator.com/item?id=30798520), 2022-03-25
- "personally i hate when i have to open a web browser to view documentation. it's an absolute waste with how heavy they are and the interface sucks." — [43632008](https://news.ycombinator.com/item?id=43632008), 2025-04-09
- "Makes you wonder what's the point of this “choice”." — [45125159](https://news.ycombinator.com/item?id=45125159), 2025-09-04 (replying to "Apple's docs are JavaScript-rendered")
- "Their search is so terrible I wish it just redirected to duckduckgo." — [41527103](https://news.ycombinator.com/item?id=41527103), 2024-09-13 (Apple's Swift docs)
- "The current web site has terrible search, has lots of links broken and the content seems mainly auto generated." — [19967795](https://news.ycombinator.com/item?id=19967795), 2019-05-21 (Microsoft)
- "the first mention of the official documentation will be half a page down with some random version." — [36141283](https://news.ycombinator.com/item?id=36141283), 2023-05-31
- "the version dropdown is a godsend especially for .NET" — [25047877](https://news.ycombinator.com/item?id=25047877), 2020-11-10
- "the feature where you can switch ES version without leaving the page you're on is a nice touch too" — [17406399](https://news.ycombinator.com/item?id=17406399), 2018-06-27
- "Discord is a great replacement for the chat app part of a support channel, it is not even a passable replacement for a website, or a wiki." — [36749777](https://news.ycombinator.com/item?id=36749777), 2023-07-16
- "There's also an awful lot of important information tucked away in WWDC videos, unfortunately." — [19971514](https://news.ycombinator.com/item?id=19971514), 2019-05-21
- "I prefer it if people just write it down so I can ctrl-f or find it in a web search and get what I need instead of sitting through your videos." — [27009209](https://news.ycombinator.com/item?id=27009209), 2021-05-01
- "Redis command docs are excellent and you can try the commands on the site itself!" — [34821775](https://news.ycombinator.com/item?id=34821775), 2023-02-16
- "Every time I see an error in terminal with a link to documentation I'm delighted. And surprised." — [33261836](https://news.ycombinator.com/item?id=33261836), 2022-10-19
- "I got an INVALID_FIELD error yesterday and was happy to see a clear message along with the hyperlink in the error object." — [34821806](https://news.ycombinator.com/item?id=34821806), 2023-02-16 (Plaid)
- "No dark mode? Seriously?" — [41346457](https://news.ycombinator.com/item?id=41346457), 2024-08-25; against: "I have astigmatism and loathe dark mode." — [36810726](https://news.ycombinator.com/item?id=36810726), 2023-07-21

### Inferences
- Single-page or downloadable forms, a fast local search and stable per-version URLs answer several recurring complaints at once (findability, offline, version confusion), and cost less than interactive features.
- Error codes that link to their docs page are rare enough that commenters are "surprised" by them; this is a cheap differentiator.

### Gaps
- No reliable count for "slow/JS-heavy docs sites", "version mismatch" or "copy button"; these rest on a handful of quotes.

## What changed in 2023–2026: AI, doc bots, llms.txt and AGENTS.md

### Takeaway
AI went from almost absent (0.2 % of 2015–2022 comments) to 8.7 % of 2023–2026 comments, and 29 of the 90 threads since 2023 are about AI and docs; commenters distrust doc chatbots that hallucinate, dislike AI-written docs, are split on llms.txt, and repeatedly note that docs written for agents are just good docs.

### Cited Findings
- Comments naming AI, LLMs, ChatGPT, Copilot, Claude, agents and similar: 18 in 2015–2022 (0.2 %) against 831 in 2023–2026 (8.7 %); 84 threads — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- Thread groups: 29 AI/agent/llms.txt stories, all from 2023 on, with 3,466 comments (16.1 % of the whole sample): doc assistants (DocsGPT, Supabase Clippy, "AI for AWS Documentation", "Copilot for Docs"), "Llms.txt" (175 comments), "AGENTS.md – Open format for guiding coding agents" (382), "Agents don't need memory, they need documentation" (312) — [story_groups.txt](../../../../tools/hn-research/developer-docs/data/story_groups.txt), [41439983](https://news.ycombinator.com/item?id=41439983), [44957443](https://news.ycombinator.com/item?id=44957443), [49945933](https://news.ycombinator.com/item?id=49945933)
- Docs for agents, llms.txt, AGENTS.md, MCP: 477 comments in 27 threads, 474 of them 2023–2026 (5.0 % of that period) — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- Distrust of AI answers and hallucination: 100 comments in 32 threads, all 2023–2026 (~6/8 on theme); AI-generated docs called "slop" or worse: 43 comments in 14 threads (~5/8) — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- "I use AI instead of reading docs" could not be counted (rule precision 1/8); quotes only.
- Themes that did not change share between periods: missing/outdated (2.1 % → 1.9 %), examples (strict 2.3 % → 2.2 %), offline/plain (1.2 % → 1.1 %), findability (1.9 % → 2.0 %). Stack Overflow as a docs source fell (strict rule: 48 comments before 2023, 16 after) — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt), [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt)

Quotes:
- "Quite a few commenters say that they're resorting to LLMs, not because that's how they want to learn about the code they're using, but because the documentation is terrible" — [38905345](https://news.ycombinator.com/item?id=38905345), 2024-01-07
- "I just ask GPT all my Postgres questions now." — [36347291](https://news.ycombinator.com/item?id=36347291), 2023-06-15
- "Reading imaginary hallucinated documentation is just about as useful as zero documentation." — [41894976](https://news.ycombinator.com/item?id=41894976), 2024-10-20
- "Docs are reliably fixable, so with enough effort they will converge to correctness. Doc bots are not and will not." — [44509871](https://news.ycombinator.com/item?id=44509871), 2025-07-09
- "If it can hallucinate an answer, it's docs with probably the most infuriating UX one can imagine." — [44507502](https://news.ycombinator.com/item?id=44507502), 2025-07-09
- "making the docs AI-friendly essentially just ends up being what good documentation is in the first place" — [44311346](https://news.ycombinator.com/item?id=44311346), 2025-06-18
- "Most of the things described in this document are also good for humans." — [44312849](https://news.ycombinator.com/item?id=44312849), 2025-06-18
- "LLM generated docs are some of the worst because like I said, the LLM never trims, only amends." — [49959682](https://news.ycombinator.com/item?id=49959682), 2026-10-05
- "READMEs should be succinct and be like a switchboard to other docs (much like LLMS.txt tries to be for agents)." — [50042925](https://news.ycombinator.com/item?id=50042925), 2026-10-11
- "I'm just left wondering who would volunteer to make their sites easier to scrape." — [41440067](https://news.ycombinator.com/item?id=41440067), 2024-09-03 (llms.txt)
- "Instead of putting resources in the root of the web, this is what /.well-known/ was designed for." — [41441178](https://news.ycombinator.com/item?id=41441178), 2024-09-04 (llms.txt)

### Inferences
- For a library whose users already use coding agents, the HN view supports publishing plain Markdown versions and an llms.txt index as a by-product of good human docs, but not an on-site chatbot answering in the docs' name: hallucinating doc bots are seen as worse than no docs.
- AI-written docs are judged harshly; human-checked, tested docs are a credibility signal in 2025–2026.

### Gaps
- Whether commenters who use LLMs read docs less could not be measured from comment text.

## What makes them read docs, and where they go instead

### Takeaway
HN commenters read docs when they trust them to be complete and correct and when they can find the answer fast; when docs fail they go to the source code, Stack Overflow (before 2023), and since 2023 to LLMs — and they judge a project by its docs before adopting it.

### Cited Findings
- Reading the source as a substitute: 559 comments (strict; noisy) — [strict_output.txt](../../../../tools/hn-research/developer-docs/data/strict_output.txt)
- "Nobody reads the docs / RTFM / skim": 243 comments in 81 threads (loose, not precision-checked) — [themes_output.txt](../../../../tools/hn-research/developer-docs/data/themes_output.txt)
- "Nerds don't respond to marketing; try technical documentation instead" drew 238 comments (2021): docs as the way to win technical buyers — [28182181](https://news.ycombinator.com/item?id=28182181)
- A 2026 story, "I paid people to try and follow my README" (206 comments), found readers stuck on missing steps and on READMEs that don't say what the software does — [50042219](https://news.ycombinator.com/item?id=50042219)

Quotes:
- "There is no faster way to lose me as a customer than a shoddy documentation." — [28195437](https://news.ycombinator.com/item?id=28195437), 2021-08-16
- "If the dependency is badly documented, then that’s a “red flag” that it may not have much of a future." — [23800720](https://news.ycombinator.com/item?id=23800720), 2020-07-11
- "forgetting to actually say what the software does is super common as well though." — [50042490](https://news.ycombinator.com/item?id=50042490), 2026-10-11
- "There's a zeroth step missing from both the Quickstart and Full Set Up" — [50042407](https://news.ycombinator.com/item?id=50042407), 2026-10-11
- "I prefer documents too. But the fact is, the choice is between nothing at all and videos." — [27009898](https://news.ycombinator.com/item?id=27009898), 2021-05-01

### Inferences
- The first screen of a README or docs home is where adoption is decided: say what it does, then a complete quick start with no missing step.

### Gaps
- No direct evidence on how long readers stay or which pages they open; HN comments describe intent, not behaviour.
