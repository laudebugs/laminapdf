# Writing docs people enjoy reading

Lamina's docs exist so that people can build things with it. These rules apply to every page a reader of the docs sees:

- the docs site (`apps/docs/src/content/docs/**`);
- the error-code pages (`docs/errors/*.md`, which the site renders);
- package READMEs (TypeDoc renders `packages/sdk/README.md`).

They build on the writing style in [design language §9](../design/paper/design-language.md#9-writing-style). Draft with the `/writing` skill and review with `/writing-review` ([git workflow §7](git-workflow.md#7-review-and-merge-process)); where a finding conflicts with a rule here, the rule here wins. The [quality plan](../plans/quality.md) records why these rules exist.

**MUST**/**SHOULD** are rules.

## Tell a story

- **MUST:** add every new page to the explicit sidebar in `apps/docs/astro.config.mjs` (no `autogenerate`). The sidebar runs in this order:
  1. start: what lamina is, install, your first PDF;
  2. your runtime;
  3. concepts;
  4. tasks (create, read and edit);
  5. production;
  6. recipes;
  7. troubleshooting;
  8. reference.
- **MUST:** open every guide and recipe with one paragraph of at most 50 words that says what you'll make and when you'd need it. Show the code next, then the details.
- **SHOULD:** end a page with a link to the page a reader needs next.

## Make it a pleasure to read

- **SHOULD:** build something a reader recognises: an invoice, a report, a form, not `foo` and `bar`.
- **SHOULD:** show the result early: the PDF it makes (a screenshot or the reference PDF), the text it reads or the error it prevents.
- **SHOULD:** talk to the reader as "you", in the present tense, the way a colleague explains something at their desk.
- **SHOULD:** keep each code block runnable and short enough to read without scrolling. Longer code belongs in a recipe.

## Write in plain English

- **MUST:** give every page a `title` and a `description` of at most 160 characters in its frontmatter.
- **MUST:** keep every sentence to 40 words or fewer.
- **SHOULD:** keep sentences to 30 words or fewer.
- **MUST:** keep paragraphs to 100 words or fewer.
- **MUST NOT:** mention ADR numbers, todo IDs, "Round", "fast lane", "golden", CI or repository paths in reader text. Contributors find those in `docs/`.
- **MUST:** explain a spec term where it first appears on a page, link it to the glossary (`/reference/glossary/`), or list it under `prose.allow` in the page's frontmatter. Move spec detail into a "Details" aside rather than deleting it.
- **MUST:** use one term per thing.
  - Write "Cloudflare Worker" and "Web Worker" in full every time.
  - Call a byte-exact reference output a "reference PDF", never a "golden".
  - Call the standard fonts "the standard 14 fonts".
- **MUST:** write headings in sentence case.
- **MUST NOT:** use "simply", "just", "easy", "obviously", "note that", "please", "etc.", "via", "utilize" or exclamation marks (outside quoted text, such as what an example draws).

## Show code

- **MUST:** put a sentence that says what the code does before every `<Snippet>`.
- **MUST:** show code only from a tested example file; [examples.md](examples.md) has the rules for those files.

## Reviewer checklist

- Could a newcomer follow this page from the one before it to the one after it?
- Does the first paragraph say what you'll make and why you'd want it?
- Would a reader enjoy it: a real task, an early result, a friendly voice?
- Is every term explained or linked where it first appears?

## How it's checked

`cargo xtask check docs-prose` runs inside `cargo xtask check docs` on every PR, docs-only ones included. It checks the MUST and MUST NOT rules under "Write in plain English" and the opening paragraph of guides and recipes. Of the two "Show code" rules, it checks the sentence before each `<Snippet>`; the other needs no check, because the docs site's `Snippet` component already refuses code from anywhere but the tested example files. The sidebar order, the SHOULD rules, the reviewer checklist and the "pleasure to read" advice stay with reviewers. The check's word lists (banned words, glossary terms, spellings, proper nouns, words that look like IDs but aren't) live in `tools/xtask/docs-prose.toml`. The check recognises todo IDs only with their letter suffix (`B8b`), because bare ones read like paper sizes, quarters and revisions (`A4`, `Q3`, `R5`).

The check is strict on every file a PR adds or changes, so new writing is checked as it is written. It only warns on untouched files until the rewrite todos have cleared them; after that it is strict everywhere. Run it while drafting with `cargo xtask check docs-prose`, or with `--warnings` to list every report and `--all-strict` to treat them all as errors. It needs no build beyond xtask and takes about a second.
