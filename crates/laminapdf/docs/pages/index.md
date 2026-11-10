---
title: "Lamina PDF"
description: "Create, read, edit and save PDFs from Rust and TypeScript, with one Rust core that runs natively and as WebAssembly."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/"
---

# Lamina PDF

Create, read, edit and save PDFs from Rust and TypeScript, with one Rust core that runs natively and as WebAssembly.

Lamina PDF creates, reads, edits and saves PDFs. One Rust core does the work, and it runs natively and as WebAssembly in browsers, Node, Deno, Bun and Cloudflare Workers. It's for apps that make documents, such as invoices, reports and tickets, and for apps that open documents other people send.

- **[`laminapdf`](https://crates.io/crates/laminapdf)** on crates.io is the Rust crate. Its [API reference](../pages/reference/rust.md) is on docs.rs.
- **[`@laminapdf/sdk`](https://www.npmjs.com/package/@laminapdf/sdk)** on npm is the TypeScript API over the WebAssembly build. After one `await init()`, every call is synchronous, and the API is the same in every runtime. Its [API reference](https://docs.laminapdf.com/reference/sdk/readme/) is on this site.

> **Alpha releases:** Both packages are alpha releases, so name the version when you install them: `npm install @laminapdf/sdk@0.2.0-alpha.5` and `cargo add laminapdf@=0.2.0-alpha.5`. [Install](../pages/install.md) lists every published package.

## What works today

| Area | What you get | Guide |
| --- | --- | --- |
| Generate | Pages, text with shaping, bidi and line breaking, embedded and subset fonts, the standard 14, vectors, colour, images, tagged PDF | [Fonts and text](../pages/guides/fonts-and-text.md), [Images and vectors](../pages/guides/images-and-vectors.md) |
| Navigate | Links, named destinations, outlines, page labels, Info and XMP metadata | [Links, outlines and metadata](../pages/guides/links-outlines-and-metadata.md) |
| Edit | Ops in transactions, undo and redo, incremental or full saves that respect signatures | [Edit a PDF](../pages/guides/edit-a-pdf.md), [Save a PDF](../pages/guides/save-a-pdf.md) |
| Read safely | Repair of damaged files as warnings, resource limits, strict mode, passwords | [Open password-protected files](../pages/guides/password-protected-files.md), [Open files you don't trust](../pages/guides/untrusted-files.md) |
| Errors | One coded error and warning shape on every surface | [Errors and warnings](../pages/concepts/errors-and-warnings.md) |

Every code block on this site is a tested example file, so the code you see is the code that runs. Each example writes the same bytes in Rust and in every JavaScript runtime.

## Where to go next

Start with [your first PDF](../pages/start/first-pdf.md), then read [how Lamina PDF thinks about a PDF](../pages/concepts/how-lamina-thinks.md). When you're ready for your own setup, pick [your runtime](../pages/quick-start/browser.md).
