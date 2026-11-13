# Docs examples: see the result, then try a change

**Status:** Draft, 2026-11-08. Research: [showing what an example makes](../research/docs-site/live-examples.md).

A recipe's reader should see what its code makes without leaving the page, and change the code where a change teaches something. This plan gives every example that makes a document a Result tab beside its code, with pictures of each page and the link to the PDF, and adds an "Edit and run" button to the few TypeScript examples worth editing. Examples that set up a runtime or read a file get no preview, since there is nothing to see.

## 1. Which examples get what

An example gets a preview only when it makes something a reader can look at. Three kinds follow from that rule:

- **It makes a PDF:** a Result tab after the TypeScript and Rust tabs, in one row, with pictures of every page and the link to open the PDF. The Rust and TypeScript twins write the same bytes, so one Result serves both languages.
- **It turns a PDF into text:** Code and Output tabs, with the text it prints, taken from its golden.
- **It sets something up, or checks something:** code only. A Worker's `wrangler.toml`, a test helper or a worker client has no output worth showing.

"Edit and run" goes only on TypeScript examples that make a PDF, are short, and teach something when one value changes. It leaves out the 100-page report (heavy to run), examples that need an input PDF, and the PDF/A ones, whose output looks the same whatever you change.

| Pages | Example | Preview | Edit and run |
| --- | --- | --- | --- |
| Recipes: Hello, world; Quick starts (Node, Deno, Bun, browser, Rust); Start: Your first PDF; Concepts: Pages and coordinates | `hello`, and the quick starts, which write the same bytes (`cargo xtask verify-install` checks it) | Result | Yes, on the recipe and wherever the Node quick start shows (Your first PDF, Quick start: Node, Pages and coordinates): the text, size and page size |
| Recipes: Invoice | `invoice` | Result | Yes: the line items |
| Recipes and Guides: Images and vectors | `images_and_vectors` | Result | Yes: the shapes and colours |
| Recipes: Links and outlines; Guides: Links, outlines and metadata | `links_and_outlines` | Result | Yes: the outline and page labels |
| Recipes: Element flow | `element_flow` | Result | Yes: the body text, to watch it flow onto page 2 |
| Recipes: PDF/UA report; Guides: Accessible PDFs | `pdf_ua_report` | Result | Yes: remove the alt text and see PDF/UA refuse it |
| Recipes: Multilingual; Guides: Text and fonts | `multilingual` | Result | No: four Noto fonts to download |
| Recipes: PDF/A archive; Guides: Archival PDFs | `pdf_a_archive` (2b and 2u) | Result | No |
| Recipes: PDF/A-3 invoice with XML; Guides: Archival PDFs | `pdf_a3_invoice_with_xml` | Result | No |
| Recipes: Generate then edit; Guides: Edit a PDF, Save a PDF; Concepts: How Lamina PDF thinks | `generate_then_edit` | Result | No: the edit is the point, and it reads like a diff |
| Recipes: Extract Markdown | `extract_markdown` | Output: the Markdown it prints for its input | No: it needs an input PDF |
| Recipes: Report | `report` | None yet: its test checks a hash, and no golden PDF exists to picture | No: 100 pages |
| Recipes: Cloudflare Worker, Web worker client, Open untrusted; Quick start: Workers; Guides: Untrusted files, Password-protected files, Test generated PDFs, Keep the page responsive; Concepts: Errors and warnings; Start: Coding agents | `worker`, `worker-client`, `open-untrusted`, `snapshot-test`, `compare-pages`, `AGENTS.md` | None | No |

## 2. Design

### Result and Output tabs (B7d)

An `Example` component in `apps/docs` wraps the language tabs that pages already use, and adds a Result or Output tab when the example has one. The Result tab shows each page as an image with its alt text and page number, the link to open the PDF, and the PDF's size. Pictures load lazily (`loading="lazy"`), so a page with five examples costs nothing until a reader opens a tab.

Since Q4d, the first page of an example's PDF shows without a click, with the link and size under it. Like the tab's pictures, it loads lazily, as it nears the window. CSS alone places it: beside the code on screens 90rem (1440 px) and wider, below it on narrower ones. Side by side, the example grows into the margins around the content column and keeps 36rem (about 66 characters) for the code. A narrower screen would squeeze the code, so there the page goes below it. The Result tab stays only where there is more than that first page: two or more pages, or two PDFs.

Since Q4q, a divider sits between the code and the page wherever they are side by side. Readers drag it, or focus it and press the arrow keys, to give the page or the code more room. The code keeps at least 20rem and the page at least 12rem. Double-click or Enter puts back the 15rem default. The width applies to every example and stays in the browser's storage for the next page. With JavaScript off, the page keeps its 15rem and shows no divider. A run draws its pages at the column's width in device pixels, between 832 and 2,000, and draws them again when the divider widens the column, so the pages stay sharp.

Since Q4r, every code block has a "Wrap long lines" button beside Copy. It wraps the lines of every block on the page at once, and the browser's storage keeps the choice for the next page. The docs build every block with Expressive Code's `wrap` and `preserveIndent`, so a wrapped line continues under its own indentation. Until a reader presses the button, the lines stay unwrapped and scroll.

Since Q4r, "Edit and run" also has a Format button at the right end of its bar. Format runs the reader's TypeScript through Prettier with the examples' settings (Biome's: 2 spaces, 120 columns), so it leaves an unedited example exactly as written. Prettier loads on the first press and never before. Code that doesn't parse stays as typed, and the bar says where Prettier stopped.

The pictures come from the example goldens, rendered at 144 dpi (2× the render goldens' 72 dpi) by the render tooling in `tools/render` (hayro), as PNG. They are committed to `apps/docs/public/examples/preview/`, next to a manifest of each golden PDF's SHA-256. `cargo xtask check docs` compares the manifest with `fixtures/golden/examples/`, so a changed example can't ship with old pictures; regenerating them is one command. The docs build stays as it is: no WebAssembly, no Python.

### Edit and run (B7e)

An "Edit and run" button loads, on first press, a small runner of our own. Since Q4d the button sits under the first page's picture; the editor takes the code's place, and a run's pages take the picture's:

- **Editor:** CodeMirror 6 (MIT) with TypeScript highlighting, holding the example's own file. Since Q4s, its lines start unwrapped and scroll sideways, as in an editor; a Wrap lines toggle beside Format, at the bar's far end, wraps them, and pressing it again unwraps them.
- **Types:** stripped by Sucrase (MIT) in the runner, with no type check, so a typo shows as the error the SDK throws, not a compiler message.
- **Run:** in a module worker the docs site serves at `/run/worker.js`. A worker's CSP comes from its own script's response, so `_headers` gives `/run/*` a policy of its own with `'unsafe-eval'`, and every page keeps ADR 0011's headers unchanged. The worker loads the docs' own build of `@laminapdf/sdk` and the fonts, images and data the example takes, all from the docs site, and passes the same fixed file ID and date as the SDK's tests, so an unchanged example gives its golden's bytes.
- **Result:** `@laminapdf/render-hayro` draws each page in the worker, the same renderer as the Result tab's pictures, and the page shows them as canvases where the picture was. A Download button saves the PDF. An error shows its code and hint, as `LaminaPdfError` gives them.
- **Reset:** puts the example's file back.
- **Limits:** a run stops after 30 seconds, so an endless loop can't hang the page, and draws at most 20 pages.

Only the first press loads anything: the editor (CodeMirror), then, on the first run, about 3 MB gzipped (the SDK's text module and hayro). Rust tabs keep the Result pictures, with no button.

The first draft of this plan ran the code in a sandboxed iframe and drew it with `@laminapdf/viewer` (PDFium). The worker needs neither: it runs only the docs' own examples, as edited by the reader in their own tab, and its separate CSP keeps `'unsafe-eval'` off every page. hayro also keeps the drawn pages identical to the pictures and ships no third-party engine.

## 3. What this plan leaves out

- **Hosted sandboxes** (StackBlitz, CodeSandbox, Sandpack): they break the site's headers and ADR 0011's same-site rule, and run the npm release instead of the docs' version ([research](../research/docs-site/live-examples.md)).
- **Running Rust in the browser.**
- **A playground page** with any code, shared links or saved snippets: the runner edits a page's own example only. A playground could reuse it later.

## 4. Todos

| Todo | Delivers |
| --- | --- |
| [B7d](../todos/done/B7d-docs-example-results.md) | The `Example` component, Result and Output tabs, page pictures with their manifest and check, every page in §1 moved to it |
| [B7e](../todos/done/B7e-docs-edit-and-run.md) | "Edit and run" for the six examples in §1: editor, runner worker, pages drawn by hayro, download, reset |
| [Q4d](../todos/done/Q4d-result-beside-the-code.md) | The first page beside or below the code with no click, "Edit and run" under it, and "Your first PDF"'s Node quick start runnable (its `save` and `writeFile` hand the runner the PDF) |
| [Q4q](../todos/done/Q4q-resizable-page-beside-the-code.md) | A divider between the code and the page beside it, dragged or moved by keys, its width kept for every example |
| [Q4r](../todos/done/Q4r-wrap-and-format-code.md) | A "Wrap long lines" button beside every code block's Copy, and Format in "Edit and run" (Prettier, loaded on the first press) |
| [Q4s](../todos/done/Q4s-editor-line-wrap.md) | The editor's lines unwrapped by default, with a Wrap lines toggle in the runner's bar |

## 5. To-dos

- [x] Show every example's result beside its code ([B7d](../todos/done/B7d-docs-example-results.md))
- [x] Let readers edit and run the TypeScript examples that teach by changing ([B7e](../todos/done/B7e-docs-edit-and-run.md))
