---
name: lamina-story
description: >
  Writes one short, verified story about Lamina PDF for social media: a LinkedIn post of 150–200
  words, a shorter version for X and Threads, and at least one Paper-styled image (a chart, drawing or diagram) for each, in each platform's size, from one part of
  the project's real history (a bug, a decision, a benchmark, how the work is done with AI agents).
  Use it when the owner asks for a post, a story, a LinkedIn or X update, or "something to share"
  about Lamina PDF, or asks which stories are left to tell. It reads and updates the coverage map in
  docs/stories/README.md so no part of the project is told twice. It never posts anything.
---

# Lamina PDF stories

Each run makes one story: one moment from Lamina PDF's history, told so a reader who has never heard of it cares how it ends. The story goes out three ways: a LinkedIn post, a short post for X, and a short post for Threads. Each carries at least one image in the Paper look that shows what the words can't. Every fact in it is checked against the repository before the owner sees it, and the owner posts it by hand.

## The brief

This is the owner's request, written out once so every run serves it:

> I'm building Lamina PDF, a platform for everything PDF, with AI agents doing much of the work. I want to tell its story, one part at a time, for two reasons: to market the product, and to share what I learn building it this way.
>
> Each post tells one story. It pulls the reader in, makes them care about the work, and leaves them wanting to try Lamina PDF or follow along. Topics range from the product (how fast it is, why that matters, what makes it fast) to the process (how we triage a bug, prove the fix and record it, or how agents work from a todo board).
>
> For each story, write:
> - a LinkedIn post of 150–200 words;
> - a version for X (one post, 280 characters at most) and one for Threads (500 characters at most);
> - at least one image with every post, in each platform's size, in Lamina PDF's own Paper look. The images show what words explain badly: a chart of a measurement, a drawing of what went wrong, a diagram of how something works. Words on an image only label it. Each image is readable at a glance, says one thing, and makes people want to read the post.
>
> Everything must be simple enough for a non-expert and technically correct. Check every claim against the repository and show me the evidence. Keep a record in the docs of which parts of the project the stories have covered, so none is told twice and I can see what is left.

## Rules that never bend

- **Lamina PDF**, never "Laminar". The packages are `@laminapdf/*`, the site `laminapdf.com`.
- **Never post, schedule or send anything.** The skill writes files; the owner publishes.
- **The prose is written with `/writing`, reviewed with `/writing-review`, and fact-checked**, every time: the three posts, the alt text and the words on the images. Load `/writing` before drafting; run `/writing-review` on the draft and fix its findings; then check every claim against the repository (step 6). A story that skipped one of the three isn't ready, whatever its deadline.
- **Every fact comes from the repository** and has a row in the story's fact table: a number, a name, a date, a "first", a comparison. A fact you can't source is cut or turned into the owner's opinion, in the owner's words.
- **Nothing private.** No secrets, no internal hostnames, no people's names other than the owner's, no customer or user data. The repository is private: write about what it shows, never link into it, and name a file only on an image's footer (where it shows the claim has a source).
- **The owner's own life** (why they started, how they feel) is theirs to tell. Write a placeholder and ask; don't invent a feeling or an anecdote.
- **Claims about AI** are claims too. "Agents review every PR" needs its source (the working agreements in `CLAUDE.md`, a merged PR). No hype about what AI did; say what it did.

## Steps

### 1. Read the coverage map and choose the story

Read [`docs/stories/README.md`](../../../docs/stories/README.md). If the owner named a topic, find its row (add one if it's new). If they didn't, offer three topics from rows marked `open`, each as one line: the hook, the turn, and why a reader would care now. Let the owner pick. Never pick a topic marked `told` unless the owner asks for a new angle, and then write the angle in the row.

### 2. Gather the facts

Read the topic's sources in the map, then follow their links: the ADR, the bug's `detail.md` and `resolution.md`, the spike's README, the benchmark data, the merged PRs (`git log`). Write every candidate fact into the fact table as you go (see the template), with the file and line or the command that shows it. Prefer the numbers the repository already measured (`apps/docs/src/data/benchmarks.json`, a bug's resolution, a spike's README) over running anything new. If a fact needs a fresh measurement, run the repo's own command (`cargo xtask …`) and record the command, the date and the machine.

### 3. Find the story

A story is someone wanting something, something in the way, and a turn. Before writing, fill these five lines in the story file:

- **Reader:** who scrolls past it (a developer who has fought PDFs, a founder building with AI, someone curious about Rust) and what they already know.
- **Hook:** the first line. It has to work on its own, because LinkedIn shows about two lines before "…see more". A surprise, a number, a question, or a moment ("At 100 % the page looked sharp. At most other zoom levels, a band of it stayed blurry."). Even an example's facts need sources: that one is "What the user saw" in `docs/bugs/viewer-blurry-at-zoom/detail.md`.
- **Tension:** what was wrong, hard or unknown.
- **Turn:** the finding or decision that changed it, with one concrete detail.
- **Payoff:** what it means for the reader, and the one thing to do next (try it, follow along, read the docs).

If any line is weak, choose another angle before writing prose.

### 4. Write the three posts

Load the `/writing` skill before the first word, and follow it for all three posts and the alt text. Then:

- **LinkedIn (150–200 words).** Hook in the first line, then short paragraphs of one to three sentences. Plain words: explain any term a non-expert would trip on, or cut it. One idea; no list of features. End on the payoff and one call to action. At most three hashtags, on the last line. First person: the owner is telling it.
- **X (≤ 280 characters, counted with the link).** The hook and the turn. One link at most.
- **Threads (≤ 500 characters).** The hook, the tension in one sentence, the turn, the payoff.
- **Alt text** for each image: what it shows and the number or words on it, in one or two sentences.

Count the words and characters with a command, not by eye, and write the counts in the story file.

### 5. Make the images

Every post carries at least one image, in its platform's size:

| Platform | Size | Template |
| --- | --- | --- |
| LinkedIn | 1080 × 1350 (4:5) | [`templates/card-portrait.html`](templates/card-portrait.html) |
| Threads | 1080 × 1350 (4:5), the same file as LinkedIn's | the same |
| X | 1600 × 900 (16:9) | [`templates/card-landscape.html`](templates/card-landscape.html) |

A post may carry more than one image (a sequence that builds the story: the symptom, the cause, the fix), but each image stands on its own, and each exists in both sizes.

**The image explains; words only label it.** Decide first what the reader should *see*: the thing that went wrong, the size of a difference, the shape of a process. Then pick the figure that shows it:

- `.bars`, a chart: for a measurement or a comparison. Lengths come from the fact table's numbers on a linear scale from zero (`--v` is each value over the largest), with the value printed beside each bar. Never a truncated axis or a log scale without saying so; a bar too small to see is the point, so let it be small.
- `.drawing`, an SVG illustration or diagram: for what happened or how something works. A page and its tiles, a document's revisions stacked, text hidden under a black box, a pipeline of three stages. Draw with the template's classes (`page`, `ink`, `rule`, `good`, `bad`, `label`), which take their colours from the `--paper-*` tokens. Keep it to the few shapes the story needs, and label parts directly rather than with a legend.

Copy the template into the story's `visuals/` folder as `<name>-portrait.html` and `<name>-landscape.html` (one pair per image), keep one figure and delete the other, and replace the example with the story's own. Fill in the kicker (the topic in two or three words), the headline (ten words at most), and the footer's source.

The rules for every image:

- **Glanceable.** On a phone, a reader gets it in two seconds: one idea, the figure taking most of the card, no paragraph, no code, about twenty words in all.
- **Correct.** Every number, label and proportion matches the post and the fact table exactly. A drawing that simplifies (four tiles for a page cut into hundreds) must not mislead: say "simplified" in the caption when the reader could take it literally.
- **On brand.** The look lives in one style sheet, `apps/site/scripts/story-card.css`, which every card links as `/story-card.css`; change the look there, never in a story's HTML. Colours only from `--paper-*` tokens (the [design language](../../../docs/design/paper/design-language.md)): never a hex value. `danger` marks what went wrong, `accent` marks Lamina PDF or what works.
- **No broken units.** Put a non-breaking space between a number and its unit (`125&nbsp;%`, `19&nbsp;KB`).

Render them from the repository root:

```bash
pnpm --filter @laminapdf/site story-image docs/stories/<story>
```

The script fails when a headline has more than ten words or the content spills past the image or its card; shorten the words rather than shrinking the type. Then open every PNG and look at it: does the figure show the story without the post, does it match the post, does anything crowd an edge?

### 6. Verify

The story isn't ready until all of these hold, and the story file records each one:

1. **Every fact has a source.** Each claim in the posts and on the images has a row in the fact table, with the file and line or the command and its output.
2. **A second reader tried to break it.** Start a fresh subagent with no context but the story file and the repository. Ask it to refute each claim from the sources: is the number the same, does the source say what the post says, does the post overstate it ("always", "first", "fastest")? Fix what it finds, and record its verdict per row.
3. **The prose passes `/writing-review`.** Run it on the three posts and the alt text, fix every finding, record how many there were and what changed in the story file's checks, then count the words again. If a fix changed a claim, check that claim again.
4. **The limits hold.** LinkedIn 150–200 words; X ≤ 280 characters; Threads ≤ 500 characters; the images render without the overflow error.
5. **Nothing private** (see the rules above).

### 7. Record it and hand it over

- The story's folder is `docs/stories/<NNN>-<slug>/` (the next number in the map): `story.md` from [`templates/story.md`](templates/story.md), `visuals/<name>-portrait.html` and `visuals/<name>-landscape.html` for each image, and their PNGs.
- In the coverage map, set the topic's **Status** to `drafted` with a link to the story (`drafted: [001](001-slug/story.md)`), and add the story to the **Told** table. The story file's **Status** says the same. Once the owner has posted it, they set both to `told` with the date, or ask you to.
- Commit on a branch and open a PR. A story is an exempt, board-free PR when it touches only `docs/stories/`. Show the owner the three posts, the images and the fact table, and ask for approval or changes.
