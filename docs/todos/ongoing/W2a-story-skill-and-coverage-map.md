# W2a — Stories: the `lamina-story` skill, its images and the coverage map

| Field | Value |
| --- | --- |
| ID | `W2a` |
| Status | `ongoing` |
| Plan | [Website §9 Stories off the site](../../plans/website.md#9-stories-off-the-site) |
| Round / milestone | Round W (website) |
| Depends on | [W1b](../done/W1b-site-scaffold-and-deploys.md) |
| ADRs | — |
| Branch | `claude/gallant-hypatia-ker2pl` |
| PR title | `feat(docs): the lamina-story skill, its Paper images and the stories' coverage map` |
| Size | M (about 300 lines of skill, templates and scripts, plus the coverage map) |
| PR | [#347](https://github.com/laudebugs/laminapdf/pull/347) |

## Goal

The owner wants to tell Lamina PDF's story one part at a time on LinkedIn, X and Threads: to market it, and to share what building it with AI agents teaches. This todo gives agents one repeatable way to do it. The `lamina-story` skill writes a verified post per platform, each with at least one image in the Paper look that shows what words explain badly. A coverage map records what has been told, so nothing is told twice.

## Context

- [Website plan §9](../../plans/website.md#9-stories-off-the-site): what the stories are for and how they're made.
- `apps/site/scripts/share-image.mjs` (W1c): the site's share image, drawn in Playwright's Chromium from the site's tokens and fonts. The story images reuse its renderer rather than starting a second one ([maintainability](../../guidelines/maintainability.md): one home per concept).
- [Design language](../../design/paper/design-language.md): the tokens every image draws with.
- `.agents/skills/`: the repository's skills; `.claude/skills` links to it so Claude Code finds them.

## Scope

**In scope**

- `.agents/skills/lamina-story/`: `SKILL.md` (the owner's brief, the rules, seven steps from choosing a topic to handing over), `templates/story.md`, `templates/card-portrait.html` (1080 × 1350) and `templates/card-landscape.html` (1600 × 900), each with a bar chart and an SVG drawing figure.
- `.claude/skills` as a link to `.agents/skills`, so every repository skill lives in one folder.
- `apps/site/scripts/paper-render.mjs`: the share image's renderer, moved out so both scripts use it, with a check that refuses content past the image's edges or its card's; `share-image.mjs` on it, with pixel-identical output.
- `apps/site/scripts/story-image.mjs` and the `story-image` script: renders a story's `visuals/*.html` at the size each declares, refusing headlines over ten words; `story-card.css`: the cards' one style sheet, both shapes.
- The coverage map, seeded with the project's stories and their sources. It began as `docs/stories/README.md` and moved, with the stories, to the owner's stories folder (`~/git/lamina-stories`) on 2026-11-09.
- Docs: the website plan's §9 and todo rows, `docs/README.md`, the git workflow's exempt PRs, `CLAUDE.md`'s layout.

**Out of scope**

- Writing the stories themselves: they live in the stories folder, outside this repository.
- Posting: the owner posts by hand.

## Tasks

- [x] The skill, its templates and the link from `.claude/skills`
- [x] The shared renderer and the story renderer; the share image unchanged
- [x] The coverage map and the docs above

## Acceptance criteria

- [x] `pnpm --filter @laminapdf/site share-image` writes the same pixels as before
- [x] `pnpm --filter @laminapdf/site story-image <dir>` renders both templates, with each figure, at their sizes, and fails on content that spills past the edges
- [x] Claude Code lists `lamina-story` among the project's skills
- [x] `cargo xtask check docs` passes

## Post-merge checks

- [ ] The first story is drafted with the skill and approved by the owner.

## Tests

The scripts are run by hand, like `share-image.mjs`; this PR runs both on the templates and compares the share image's pixels.

## Docs to update

Listed in scope.

## Risks and open questions

- **A symlink on Windows.** Git checks `.claude/skills` out as a small text file unless symlinks are enabled. Only Claude Code reads it, and the skills themselves stay in `.agents/skills`.
- **Platform sizes change.** The sizes are in the skill and the templates' `lamina-size` metas; changing them is a template edit.

## Definition of done

- [x] Claimed: this file in `ongoing/` in the work PR's first commit
- [x] The change follows the guidelines that apply to it
- [ ] `/code-review` process followed, and `/writing-review` on the PR's prose
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links updated, plan to-dos ticked, board regenerated
