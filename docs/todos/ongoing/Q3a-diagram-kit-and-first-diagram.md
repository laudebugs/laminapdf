# Q3a — The diagram kit, its checks, and "Open, edit, save"

| Field | Value |
| --- | --- |
| ID | `Q3a` |
| Status | `ongoing` |
| Plan | [Docs diagrams §3, §4](../../plans/docs-diagrams.md#3-how-the-diagrams-are-drawn) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | None |
| ADRs | Follows [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no third-party loads, the docs' CSP) and [0024](../../decisions/0024-app-shell-ux.md) §7 (the `--paper-` tokens) |
| Branch | `docs/q3a-diagram-kit` |
| PR title | `feat(docs): a diagram kit that follows the theme, and "Open, edit, save"` |
| Size | M |
| PR | — |

## Goal

Build the kit every docs diagram uses: inline SVG from Astro components, coloured only by the `--paper-*` tokens, so a diagram follows the light theme, the dark theme and forced colours without code of its own. Ship it with the checks that keep every later diagram right, and prove it on the first diagram readers need: how an edit reaches the saved file, on the "How Lamina PDF thinks" page.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3 (the approach and the style) and §4 (the checks): read both first.
- [Paper design language](../../design/paper/design-language.md) §1–5 and `packages/tokens/tokens.json`: the colour, stroke, radius and type rules a diagram follows; `packages/tokens/CONTRAST.md` lists the token pairs that pass.
- `apps/docs/src/styles/paper.css`: how the docs map Starlight's colours onto the tokens per theme.
- `apps/docs/plugins/markdown.mjs` and `markdown.test.mjs`: the Markdown twins, which pass unknown components through as raw JSX today.
- `apps/site/src/components/home/Dashboard.astro`: inline SVG coloured through classes, the precedent.
- `packages/site-kit/test/browser/`: renders an Astro component with Astro's container and checks it in three browsers; the browser test here copies that setup.
- [Docs writing guide](../../guidelines/docs-writing.md): the labels and the text description follow it.

## Scope

**In scope**

- The kit, `Diagram.astro`, `diagrams.css`, the twin rule and the three checks of plan §4.
- Diagram 1, "Open, edit, save", on `concepts/how-lamina-thinks`.

**Out of scope** (and which todo covers it)

- The other diagrams: Q3b–Q3g.
- A high-contrast option in the docs' theme switch (plan §8 open question; decide in this PR's review, build it separately if wanted).

## Tasks

- [ ] `apps/docs/src/components/diagrams/kit/`: Astro components for a box (`Box`: label, optional mono sublabel, `focus` flag), a labelled arrow (`Arrow`: straight or one bend, a marker whose `id` takes a per-diagram prefix), a lane (`Lane`: a titled column or row), a byte strip (`Strip`: segments with labels) and a page outline (`PageSheet`: `page-paper` ground, `page-ink` marks). Fixed sizes and spacing on the 4/8/12/16/24 scale; strokes 1.5, borders 1, focus 2 in `accent`; radius 4.
- [ ] `apps/docs/src/components/Diagram.astro`: props `id`, `caption`, `text` (required), `viewBox`; renders `<figure class="dg">` with the SVG as `role="img"`, `aria-labelledby` (the caption) and `aria-describedby` (a visually hidden `<p>` holding `text`), and a `<figcaption>`.
- [ ] `apps/docs/src/styles/diagrams.css`, imported where `paper.css` is: the `.dg-*` classes, every colour a `var(--paper-*)`, scoped under `.dg`; `width: 100%`, no fixed height; a forced-colours block only if a class needs one beyond what the tokens already map.
- [ ] `apps/docs/src/components/diagrams/OpenEditSave.astro`: original bytes and the overlay side by side, `doc.transact`, undo and redo writing into the overlay, reads checking the overlay first, then `save` splitting into `incremental` (original bytes plus an appended revision) and `fullRewrite` (a new file). The overlay is the one `accent` element. Text description: the same in two or three plain sentences.
- [ ] Place it on `apps/docs/src/content/docs/concepts/how-lamina-thinks.mdx` under "The file you opened never changes", after that section's first paragraph; trim any sentence the picture now carries only if the page stays complete without it.
- [ ] `plugins/markdown.mjs`: a rule that turns `<Diagram …>` and any `diagrams/*` component into `*<caption>*` followed by the `text` paragraph; nothing else of it reaches the twin or `llms-full.txt`.
- [ ] `apps/docs/test/diagrams.test.mjs` (run by the package's `test` script): every file under `components/diagrams/` has no hex, `rgb(`, `hsl(` or named colour, no `style=` attribute, no duplicate `id` inside one diagram; every `<Diagram>` use in the pages has a non-empty `text`.
- [ ] A browser test (`apps/docs/test/browser/`, with a Vitest browser config like site-kit's, added to `cargo xtask pdf-runtimes`' browser suites in `tools/xtask/src/pdf_runtimes.rs`): render every diagram with Astro's container in `data-theme="light"`, `data-theme="dark"` and with forced colours emulated; run axe's `color-contrast` and `svg-img-alt`; render at 360 px wide and fail when any label's computed font size is under 12 px.
- [ ] `docs/guidelines/docs-writing.md`: a short "Diagrams" section: when to draw one, use the kit, labels of four words or fewer in the API's names, the `text` prop as the full meaning, never before the opening paragraph.

## Acceptance criteria

- [ ] The diagram renders on the built page in both themes and switches with the header's theme control without a reload; screenshots of both are in the PR.
- [ ] The page's `.md` twin and `llms-full.txt` hold the caption and the text description, and no `<svg>`, `<Diagram` or `import` line.
- [ ] The lint, twin and browser tests pass, and each fails on a planted bad diagram (a hex colour, a missing `text`, a 10 px label), shown in the PR description.
- [ ] `cargo xtask check docs` passes; the docs build passes.
- [ ] The diagram is under 8 KB gzipped (reported in the PR).

## Post-merge checks

- [ ] The diagram shows on `next.docs.laminapdf.com` in both themes.

## Tests

The lint and twin tests (Node, `pnpm --filter @laminapdf/docs test`) and the browser test (Chromium, Firefox, WebKit, through `cargo xtask pdf-runtimes`).

## Docs to update

`docs/guidelines/docs-writing.md` (the Diagrams section), `apps/docs/README.md` (where diagrams live and how to add one), the plan's §9 to-do.

## Risks and open questions

- Astro's container may not load the docs' CSS the way the built site does; if the browser test can't see the tokens, it renders a small fixture page that links `paper.css` and `diagrams.css`.
- Forced-colours emulation differs by browser (Playwright's `forcedColors` works in Chromium and Firefox); WebKit may run the two themes only, noted in the test.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
