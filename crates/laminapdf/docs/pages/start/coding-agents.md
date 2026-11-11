---
title: "Use Lamina PDF with coding agents"
description: "Point a coding agent at the docs inside the installed package, so its answers match the version you use."
version: "0.2.0-alpha.6"
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
- **Data-driven layouts go in as JSON.** `laminapdf.document/1` describes flowing pages (page, named styles, blocks tagged by `type`); `flow::document::fill(template, data)` fills its `$data`, `$each`, `$item` and `$if` placeholders, and `Builder::document` lays it out (Rust; TypeScript follows). Check a document you write against its schema, which this package ships as `docs/schemas/document.schema.json`. Put totals and dates in the data already formatted: templates have no expressions.
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
