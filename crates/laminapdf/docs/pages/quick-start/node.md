---
title: "Node"
description: "Generate a PDF from Node with @laminapdf/sdk."
version: "0.4.0"
url: "https://docs.laminapdf.com/quick-start/node/"
---

# Node

Generate a PDF from Node with @laminapdf/sdk.

In Node, `@laminapdf/sdk` reads its `.wasm` files from the `@laminapdf/wasm` package as bytes. Call `await init({ text: true })` once; after that, every call is synchronous.

This app builds one A4 page that says "Hello, world!" and writes it to `hello.pdf`:

**TypeScript**

`packages/sdk/examples/quick-start/node.ts`:

```ts
// Builds a one-page PDF that says "Hello, world!" and writes it to hello.pdf, in Node.
import { writeFile } from "node:fs/promises";
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** The space between the page's edge and the text, in points (72 points = 1 inch). */
const MARGIN = 72;
/** The text's size in points; its baseline sits this far below the margin. */
const FONT_SIZE = 24;

/** Builds the page and hands its bytes to `save`, which writes hello.pdf unless you pass another `save` function. */
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = (pdf) => writeFile("hello.pdf", pdf),
  options: BuilderOptions = {},
): Promise<void> {
  await init({ text: true });
  using builder = new Builder(options);
  const page = builder.page("A4");
  // Points from the page's top-left corner, y down; the text's baseline sits FONT_SIZE points below the margin.
  page.surface().text("Hello, world!", {
    font: Font.standard("Helvetica"),
    size: FONT_SIZE,
    at: [MARGIN, MARGIN + FONT_SIZE],
  });
  page.finish();
  using doc = builder.finish();
  await save(doc.save());
}

// `node hello.ts` runs it (Node 24.2 or newer); imported, it only exports `hello`.
if (import.meta.main) await hello();
```

**Result:** [hello.pdf](https://docs.laminapdf.com/examples/pdf/hello.pdf)

Save it as `hello.ts` in a project with `@laminapdf/sdk` [installed](../../pages/install.md), and run it with Node 24.2 or newer, which runs TypeScript files as they are:

```sh
node hello.ts
```

Node 22 can't parse the `using` that frees the builder and the document, so compile the app with `tsc` first there. Lamina PDF's tests run this app in Node 22 (compiled), 24 and 26 and check that it builds the reference PDF byte for byte.

A long-running server should free documents the same way; [how Lamina PDF thinks about a PDF](../../pages/concepts/how-lamina-thinks.md#you-free-what-you-create) explains why.

Next: draw text with [fonts](../../pages/guides/fonts-and-text.md), or [open and edit](../../pages/guides/edit-a-pdf.md) a file.
