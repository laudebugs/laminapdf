# Plan: diagrams that show how Lamina PDF works, in the docs' own style (Round Q3)

- **Status:** Draft
- **Date:** 2026-11-08
- **Owner:** the docs track
- **Builds on:** [Quality](quality.md) (Round Q: docs as a story), the [Paper design language](../design/paper/design-language.md), [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md) (no third-party loads, the CSP)
- **Todos:** Round Q3 on the [todo board](../todos/README.md); §6 lists them.

## 1. Goal

The docs site has no images at all. Its pages explain processes in prose: how an edit reaches the saved file, why one save keeps signatures and another breaks them, where a point lands on a rotated page, which steps turn a PDF into Markdown. Readers have to build each picture in their head from several paragraphs, and some of them build the wrong one. Round Q3 adds twelve diagrams to the pages where a picture answers a question faster than text, drawn so they match the docs and follow the theme the reader picked. This plan names the diagrams and where they go, fixes one way to draw them that works in light mode, dark mode and forced colours, and splits the work into seven pull requests that each ship finished diagrams.

## 2. Where a diagram helps

A diagram earns its place when the page explains a flow, a structure or a geometry that the reader must hold in mind while reading the next section. Each one below answers a question readers ask, and each sits next to the prose it supports, and none decorates a page. The ranking is a judgement from reading the pages: how central the question is to using the library, and how much prose the picture saves. No usage data backs it yet; the docs collect none.

| # | Diagram | Page and section | The question it answers | Kind |
| --- | --- | --- | --- | --- |
| 1 | Open, edit, save | `concepts/how-lamina-thinks`: "The file you opened never changes", "Two ways to save" | Where do my edits live, and what does save write? | Flow |
| 2 | Incremental save and full rewrite | `guides/save-a-pdf`: "Choose a save mode" | Why does one save keep signatures valid? | Two byte strips |
| 3 | How a save picks its mode | `guides/save-a-pdf`: "When a file refuses a save" | Why did my save fail, and which option fixes it? | Decision flow |
| 4 | Page space and user space | `concepts/pages-and-coordinates`: "The top-left corner of the page you see", "PDF's own coordinates" | Where does `at: [72, 96]` land, also on a rotated page? | Coordinate figure |
| 5 | The packages and modules | `install`: "Every published package"; `concepts/how-lamina-thinks`: "Three modules in JavaScript" | Which package do I install, and what does it bring? | Layered stack |
| 6 | From page to Markdown | `recipes/extract-markdown`: "How it works" | What happens between the PDF and the Markdown? | Pipeline |
| 7 | Reading order on a two-column page | `recipes/extract-markdown`: "How it works" | In which order does the text come out? | Page sketch |
| 8 | Lamina PDF in a Web Worker | `guides/keep-the-page-responsive`: "Call the same methods, awaited", "Cancel a call" | What runs where, and what can I cancel? | Sequence |
| 9 | Errors and warnings | `concepts/errors-and-warnings`: "Errors stop the call", "Every surface has the same fields" | Did my call fail, and where are the details? | Split flow and object |
| 10 | Transactions and undo | `guides/edit-a-pdf`: "Group edits into a transaction", "Undo and redo" | What does one undo take back? | Timeline |
| 11 | Drawing a page | a new page, `concepts/drawing-pages` | Who draws the page, and who owns its text? | Flow |
| 12 | Opening a large file by ranges | a new page, `guides/open-large-files` | How does it open a remote file without downloading all of it? | Loop |

Two candidates wait. The viewer's parts (main thread, Web Worker, tiles, cache) belong on a viewer page once `@laminapdf/viewer`'s API is stable; drawing it now would document an API that still changes. Element flow (`recipes/element-flow`: parts stacked by `bottom`) is clear in its example code, so a figure adds less there than anywhere above.

Each diagram uses the names the page and the API use, exactly: `incremental`, `fullRewrite`, `data-pending`, `Document.openRange`, `doc.transact`. The labels follow the [docs writing guide](../guidelines/docs-writing.md) like the prose around them: plain English, sentence case, "Web Worker" in full, no internal names such as todo IDs, ADR numbers, "golden" or "CI".

## 3. How the diagrams are drawn

Every diagram is inline SVG that an Astro component writes at build time, coloured only through the `--paper-*` design tokens. This plan compared three ways to show a picture that follows the theme:

- **`<img>` pairs** switched by Starlight's `light:sl-hidden` and `dark:sl-hidden` classes. Each diagram becomes two files with colours written into them, forced colours aren't followed, and the text can't be selected or searched.
- **`<picture>`** with `prefers-color-scheme`, or an SVG with its own media query. Both follow the system setting and ignore the theme the reader picks in the docs' header.
- **Inline SVG coloured by tokens.** The docs' theme switch sets `<html data-theme>`, and `@laminapdf/tokens` already maps every `--paper-*` colour per theme and to system colours under forced colours. A diagram drawn with those variables follows the manual switch, the automatic mode, a future accent and forced colours without code of its own.

Inline SVG wins, and the docs already allow it: the CSP governs scripts, styles and loaded images, not markup. The website's dashboard (`apps/site/src/components/home/Dashboard.astro`) draws the same way, as a precedent. Because the website's CSP forbids inline styles, the diagrams colour themselves through classes in a stylesheet, never `style=""` attributes, so any of them could appear on the website too.

### 3.1 The parts

- **`apps/docs/src/components/diagrams/`** holds one `.astro` file per diagram and a small kit the diagrams share (`kit/`): a box, an arrow, a lane, a label, the arrowheads and the SVG wrapper, from Q3a; a byte strip (`Strip`, Q3b) and a page outline (`PageSheet`, Q3c) join it with the first diagrams that need them. The kit keeps sizes, strokes and spacing identical across diagrams, so a reader learns the visual language once.
- **`Diagram.astro`** places a diagram by name (`<Diagram name="open-edit-save" />`) and wraps it in a `<figure>` with its caption. **`diagrams/registry.mjs`** holds each diagram's caption and text description, the diagram's full meaning in prose, in one place: the page reads it for the caption and for screen readers, and the Markdown twin, which is built from the page's source and can't see inside a component, reads the same entry.
- **`apps/docs/src/styles/diagrams.css`** styles the kit's classes (`.dg-box`, `.dg-arrow`, `.dg-focus`, `.dg-label`), with every colour a `var(--paper-*)`. The classes are scoped under `.dg`, because the tokens' `:where()` rules lose to any other rule.
- **The Markdown twins.** `plugins/markdown.mjs` turns `<Snippet>`, asides and the other components into Markdown for the twins that coding agents read, and passes any unknown component through as raw JSX. A diagram therefore needs its own rule there: the twin gets the caption and the `text` prop, never the SVG.

### 3.2 The style

The diagrams follow the Paper design language: calm and editorial, chrome removed rather than colour added.

- **Colour.** Grounds are `surface` and `raised`, text is `text` and `text-2`, captions `muted`. Lines that carry meaning use `control-border` (at least 3:1); `hairline` (1.22:1) is for decoration only. The teal `accent` marks the one thing in focus, such as the overlay in the save diagram or the step the section discusses, never decoration. Status colours (`danger`, `positive`, `warning`) mean what they mean in the apps, and colour is never the only cue: a refused save also says "refused".
- **A PDF page** inside a diagram uses `page-paper` and `page-ink`, so it looks like the page the reader sees in a viewer.
- **Lines and shapes.** Strokes are 1.5 px, like the icons; borders 1 px, a focused element 2 px in `accent`. Corners use the radius tokens (4 for boxes). No shadows and no gradients.
- **Type.** Labels use `--paper-font-sans`, at least 12 px at the docs' content width, sentence case; API names use `--paper-font-mono`. SVG text doesn't wrap, so a label stays at four words or fewer and the page's prose carries the rest.
- **Layout.** Each SVG has a `viewBox`, `width: 100%` and no fixed height, so it scales to phone width; a diagram too wide to read there gets a second, vertical layout. Spacing uses the 4, 8, 12, 16 and 24 steps.
- **Flat, two-dimensional figures.** Isometric or three-dimensional drawings would fight the docs' quiet look.

### 3.3 Accessibility and placement

Each diagram is `role="img"` with `aria-labelledby` naming its caption and `aria-describedby` its text description, so a screen reader hears the same content the twin holds. Every page keeps its prose complete without the picture: a diagram supports the text and never replaces a sentence the reader needs. A diagram never comes before a page's opening paragraph, which the docs-prose check measures up to the first component.

## 4. Checks that keep the diagrams right

A diagram can go wrong in four ways. A hard-coded colour breaks in one theme, a label is too faint to read, a twin leaks raw markup, or the picture disagrees with the API. Each has a check:

1. **A lint test** (Node, `apps/docs/test/diagrams.test.mjs`) reads every diagram component and fails on a hex, `rgb()` or named colour, a `style=""` attribute, a fixed `id`, a registry entry without its drawing or a page naming a diagram that doesn't exist.
2. **A browser test** in Chromium, Firefox and WebKit renders every diagram with Astro's container, as `@laminapdf/site-kit`'s header test does, in the light, dark and high-contrast themes and with forced colours emulated (Chromium and Firefox). It computes the contrast of every label against the shape behind it and of every line against the page, since axe-core doesn't measure contrast inside SVG, and checks that the SVG is an image with a name and a description. `cargo xtask pdf-runtimes` runs it with the other browser suites.
3. **A twin test** in `plugins/markdown.test.mjs` checks that a page with a `<Diagram>` produces its caption and text, and no `<svg>` or JSX, in the `.md` twin and `llms-full.txt`.
4. **Review.** The labels and the text description get `/writing-review` like the page's prose, and the PR shows a screenshot of each diagram in both themes.

## 5. Out of scope

- Diagrams on the website (`apps/site`) and in the package READMEs. The kit works there, but those pages need their own plan.
- Screenshots of the reader app. They belong to the web reader's own docs.
- Animation and interaction. A static figure explains these processes, and motion would need a reduced-motion variant.
- Diagrams drawn by a library at build time (Mermaid, D2). Their output can't use the design tokens without heavy restyling, and hand-drawn SVG through the kit stays smaller and closer to the Paper look.

## 6. Todos

Each todo ships finished diagrams on live pages. Q3a builds the kit, the checks and the first diagram; the others depend only on it and can run in parallel. Start Q3g first after Q3a: its "Drawing a page" diagram answers how rendering works, the question that started this round, and it needs a new page, so it takes longest.

| ID | Delivers | Diagrams (§2) |
| --- | --- | --- |
| [Q3a](../todos/done/Q3a-diagram-kit-and-first-diagram.md) | The kit, `Diagram.astro`, the styles, the twin rule, the three checks | 1 |
| [Q3b](../todos/ready/Q3b-save-diagrams.md) | The two save diagrams | 2, 3 |
| [Q3c](../todos/ready/Q3c-coordinates-diagram.md) | Page space and user space, upright and rotated | 4 |
| [Q3d](../todos/ready/Q3d-package-map-diagram.md) | The packages and modules | 5 |
| [Q3e](../todos/ready/Q3e-extraction-diagrams.md) | From page to Markdown, and reading order | 6, 7 |
| [Q3f](../todos/ready/Q3f-worker-errors-undo-diagrams.md) | Web Worker, errors and warnings, transactions | 8, 9, 10 |
| [Q3g](../todos/ongoing/Q3g-drawing-and-ranges-pages.md) | Two new pages, each with its diagram | 11, 12 |

## 7. Risks

- **Diagrams drift from the API.** A renamed option leaves a picture wrong while the prose is fixed. Labels use the same names as the page, the review checks them, and any PR that renames an API item searches the diagram components as well as the pages.
- **Labels too small on phones.** A wide figure shrinks below 12 px at phone width. The browser test also renders each diagram at 360 px wide and fails when a label's computed size drops below 12 px; such a diagram gets its vertical layout.
- **Inline SVG weighs on pages.** The sample "Open, edit, save" diagram is 0.8 KB gzipped, so twelve add about 10 KB across the site, small next to the docs' fonts. Each PR still reports its diagrams' gzip size, and a diagram over 8 KB gzipped is redrawn simpler.

## 8. Open questions

- Should the docs offer the high-contrast theme in their theme switch? The diagrams would follow it for free, but the docs pages haven't been checked in it. Decide in Q3a's review; the browser test covers forced colours either way.
- Decided in Q3g: the two new pages go into the sidebar's existing groups, "How a page is drawn" under "Concepts" and "Open large files by ranges" under "Read and edit PDFs", beside "Open files you don't trust". A "How it works" group would hold two pages, and the concept pages already answer how it works.

## 9. To-dos

- [x] Q3a: the kit, the checks and "Open, edit, save"
- [ ] Q3b: the save diagrams
- [ ] Q3c: page space and user space
- [ ] Q3d: the packages and modules
- [ ] Q3e: the extraction diagrams
- [ ] Q3f: Web Worker, errors and warnings, transactions
- [ ] Q3g: the drawing and ranges pages
