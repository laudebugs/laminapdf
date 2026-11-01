# Writing docs people enjoy reading

These rules apply to every page a reader of lamina's docs sees:

- the docs site (`apps/docs/src/content/docs/**`);
- the error-code pages (`docs/errors/*.md`, which the site renders);
- package READMEs (TypeDoc renders `packages/sdk/README.md`).

They build on the writing style in [design language §9](../design/paper/design-language.md#9-writing-style) and the `/writing` skill ([git workflow §7](git-workflow.md#7-review-and-merge-process)). The [quality plan](../plans/quality.md) explains why they exist.

**MUST**/**SHOULD** are rules. *Recommendation* marks advice.

## Tell a story

- **MUST:** a page takes its place in the sidebar's order, which is fixed in `apps/docs/astro.config.mjs` (no `autogenerate`). The order runs:
  1. start: what lamina is, install, your first PDF;
  2. your runtime;
  3. concepts;
  4. tasks (create, read and edit);
  5. production;
  6. recipes;
  7. troubleshooting;
  8. reference.
- **MUST:** every guide and recipe opens with one paragraph of at most 50 words that says what you'll make and when you'd need it. The code comes next, then the details.
- **SHOULD:** a page links to the page a reader needs next.

## Write in plain English

- **MUST:** every page has a `title` and a `description` of at most 160 characters in its frontmatter.
- **MUST:** sentences stay under 40 words, and aim for under 30. Paragraphs stay under 100 words.
- **MUST:** reader text never mentions:
  - ADR numbers or todo IDs;
  - "Round", "fast lane", "golden" or repository paths;
  - CI.

  Contributors find those in `docs/`.
- **MUST:** a spec term is explained where it first appears on a page, or linked to the glossary (`/reference/glossary/`). Move spec detail into a "Details" aside rather than deleting it.
- **MUST:** use one term per thing. "Cloudflare Worker" and "Web Worker" are always written in full. A byte-exact reference output is a "reference PDF", never a "golden". The standard fonts are "the standard 14 fonts".
- **MUST:** headings use sentence case.
- **SHOULD NOT:** use "simply", "just", "easy", "obviously", "note that", "please", "etc.", "via", "utilize" or exclamation marks.

## Show code

- **MUST:** a sentence that says what the code does comes before every `<Snippet>`.
- **MUST:** code on a page comes from a tested example file; [examples.md](examples.md) has the rules for those files.

## How it's checked

`cargo xtask check docs-prose` checks every rule above except "tell a story" and the link to the next page, which stay with the reviewer. It is part of `cargo xtask check docs`, so it runs on every PR, docs-only ones included. It warns until the rewrite todos land, and fails after [Q1l](../todos/ready/Q1l-docs-checks-strict.md).

Reviewers also run `/writing-review` on changed pages ([git workflow §7](git-workflow.md#7-review-and-merge-process)). Where a writing finding conflicts with a rule here, the rule here wins.
