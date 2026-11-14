---
title: "Troubleshooting"
description: "Find the fix for what you see when Lamina PDF goes wrong, whether you use it from JavaScript, Rust or the command line, or to extract text or render pages."
version: "0.4.0"
url: "https://docs.laminapdf.com/troubleshooting/"
---

# Troubleshooting

Find the fix for what you see when Lamina PDF goes wrong, whether you use it from JavaScript, Rust or the command line, or to extract text or render pages.

Each troubleshooting page starts from what you see, such as an error code, an exit code or text in the wrong order, then gives the cause and the fix. Pick the page for what you were doing:

- [JavaScript and TypeScript](../pages/troubleshooting/javascript.md): installing `@laminapdf/sdk`, loading its WebAssembly in browsers, bundlers and Cloudflare Workers, and its memory.
- [Rust](../pages/troubleshooting/rust.md): the version to depend on, the crate's features and the Rust version it needs.
- [The command line](../pages/troubleshooting/command-line.md): exit codes, passwords, files that already exist and output to a terminal.
- [Opening and saving files](../pages/troubleshooting/files.md): passwords, signed files and files over a limit, in every language.
- [Extracting text](../pages/troubleshooting/extraction.md): empty pages, text in the wrong order, missing characters, and headers that vanish.
- [Rendering pages](../pages/troubleshooting/rendering.md): PDFium not found, images too large, and pages that draw blank or wrong.

Every error and warning carries a `code`, and every code has a page that says what it means and what to do. When an entry here doesn't match, look the code up in the [error and warning codes](../errors/README.md).
