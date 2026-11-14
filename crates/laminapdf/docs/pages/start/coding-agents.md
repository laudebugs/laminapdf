---
title: "Use Lamina PDF with coding agents"
description: "Point a coding agent at the docs inside the installed package, so its answers match the version you use."
version: "0.4.0"
url: "https://docs.laminapdf.com/start/coding-agents/"
---

# Use Lamina PDF with coding agents

Point a coding agent at the docs inside the installed package, so its answers match the version you use.

A coding agent guesses a new library's names from what it saw in training, which may be an older version or none at all. Each package therefore carries its own copy of these docs as Markdown. The copy sits in `node_modules/@laminapdf/sdk/docs/` for TypeScript and in the `laminapdf` crate's `docs/` folder for Rust, and every file names the version it describes.

## Add the snippet to your agent's instructions

Copy this snippet into your project's `AGENTS.md`, `CLAUDE.md` or the file your agent reads. It sends the agent to the installed docs first, and it lists the rules where Lamina PDF differs from other PDF libraries:

`packages/sdk/examples/AGENTS.md`:

```md
## Lamina PDF

This project uses Lamina PDF (`@laminapdf/sdk` in TypeScript, the `laminapdf` crate in Rust) to create, read, edit and save PDF files.

- **Read the installed docs first.** They match the installed version: `node_modules/@laminapdf/sdk/docs/llms.txt` lists every page, and `docs/api/` holds the API reference. In Rust they are in the `laminapdf` crate's `docs/` folder. Online: https://docs.laminapdf.com/llms.txt (the latest version).
- **Call `await init()` once before anything else.** After it every call is synchronous. Fonts and text need `await init({ text: true })`; reading text needs the extract or text module (`await init({ extract: true })` or `{ text: true }`). Rust needs no setup.
- **Positions are points from the top-left corner** of the page as it is displayed, with y going down. A text position is its baseline. Page numbers in the API start at 0 (`page(0)` is the first page).
- **Branch on an error's `code`, never its message.** Every error is a `LaminaPdfError` with a stable `code` (such as `wrong-password`) and a `hint`; `docs/errors/<code>.md` explains each one. Warnings, which say what Lamina PDF repaired in a damaged file, use the same codes.
- **Free what you open.** A `Document` or `Builder` holds WebAssembly memory: declare it with `using`, or call `free()` when you are done.
- **Check `docs/renames.json`** when a name in older code or an older answer doesn't exist: it says what each renamed item is called now.
- **Prose goes in as Markdown.** `builder.markdown(text, { fonts, images })` (Rust: `Builder::markdown`) turns CommonMark (with tables, footnotes and task lists) into tagged pages; front matter (`title:`, `lang:`) sets the metadata. Pass images by the name the Markdown uses; nothing is fetched. Without `fonts`, only Latin text can be set, and PDF/UA-1 refuses the unembedded fonts. Without code: `laminapdf build report.md -o report.pdf --font regular=FILE`, or the MCP tool `laminapdf_build` from `laminapdf mcp --root DIR --allow-write`.
- **Data-driven layouts go in as JSON.** `laminapdf.document/1` describes flowing pages (`schema`, `page`, named `styles`, `blocks` tagged by `type`). In TypeScript, `builder.flow(document, { data, fonts, images })` lays it out; type it as `FlowDocument`, or as `FlowTemplate` when it holds placeholders (`{ $data: "/ptr" }`, `{ $item: "/ptr" }`, `{ $each: "/list", do }`, `{ $if: "/flag", do }`, which nest) that `data` fills; `tsc` refuses a misspelt key in an object literal. Fonts and images go by the names its styles and figures use. Its `profiles` are not applied for you: pass `template.profiles` to `new Builder({ profiles })`. Errors: `invalid-document` (`data.pointer`, `didYouMean`), `template-data-missing` (`data.pointer`). Examples: `invoice-from-json.ts` (a template and its data), `flowing-report.ts` (a contents page, footnotes, a running header, Arabic and Hindi) and `flowing-invoice.ts` (a table across pages). In Rust, `flow::document::fill(template, data)` and `Builder::document`. Without code: `laminapdf build invoice.json --data data.json --font regular=FILE --font bold=FILE -o invoice.pdf` (one `--font NAME=FILE` per font the document names), or `laminapdf_build` with `document` and `data` inline. Check a document you write against its schema, which this package ships as `docs/schemas/document.schema.json`. Put totals and dates in the data already formatted: templates have no expressions.
- **Text comes out in reading order.** `doc.extract({ format })` needs the extract or text module. `"text"` and `"markdown"` return a string, `"json"` an `Extraction` (every block with its kind, page and box) and `"chunks"` a `Chunk[]` for retrieval. Hidden text (under a box, white on white, off the page, too small) never reaches the output; the JSON counts what was dropped. Examples: `extract-text.ts`, `extract-markdown.ts`, `extract-chunks.ts`. In Rust, `Document::extract` and `extract_json`. Without code: `laminapdf extract report.pdf --format md` (or `json`, `chunks`).
- **The `laminapdf` command does the same without code.** `info` describes a file, `check` reports what was repaired and failed redactions, `extract` writes text, Markdown, JSON or chunks, `render` writes pages as PNG, `reveal` lists what a file holds beyond its pages (earlier revisions, metadata, attachments, scripts, hidden text; `--fail-on` exits 6), `build` makes a PDF from Markdown or `laminapdf.document/1` JSON, and `pages rotate` turns pages. `--json` prints a versioned report. `laminapdf mcp --root DIR` serves `laminapdf_check`, `laminapdf_explain`, `laminapdf_extract`, `laminapdf_info` and `laminapdf_reveal` to MCP agents; `--allow-write` adds `laminapdf_build`, `laminapdf_pages_rotate` and `laminapdf_render`. `@laminapdf/sdk` has no `reveal` yet.
```

## What the package docs hold

The folder starts at `llms.txt`, an index in the [llms.txt](https://llmstxt.org/) format whose links stay inside the folder:

- `pages/`: every page of this site as Markdown, with the examples written out in full.
- `errors/`: a page per error and warning code, the same text as the [code pages](../../errors/README.md).
- `api/`: the TypeScript API reference (npm package only; Rust's is on docs.rs).
- `renames.json`: what each renamed item is called now, so an agent holding an old name finds the new one.
- `AGENTS.md`: the snippet above.

## The same docs online

This site serves the same Markdown for the latest version. Each page has a twin at its address with `.md` in place of the final slash, such as [`/install.md`](https://docs.laminapdf.com/install.md), and [`/llms.txt`](https://docs.laminapdf.com/llms.txt) lists them all. [`/llms-full.txt`](https://docs.laminapdf.com/llms-full.txt) puts every page in one file for agents that read a whole site at once. Prefer the package copy while you work in a project, because it matches the code the agent calls.

Every page also names its twin in a `<link rel="alternate" type="text/markdown">` tag, so an agent that fetches a page's HTML finds the Markdown. To paste one page into a chat, press **Copy as Markdown** at the top of the page.
