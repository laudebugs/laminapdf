---
title: "Your first PDF"
description: "Write hello.pdf from Node in five minutes."
version: "0.2.0"
url: "https://docs.laminapdf.com/start/first-pdf/"
---

# Your first PDF

Write hello.pdf from Node in five minutes.

You'll write `hello.pdf`, one A4 page that says "Hello, world!", from a short Node script. It takes about five minutes.

## Make a project

You need Node 24.2 or newer, which runs `.ts` files directly. Create a folder and install `@laminapdf/sdk` from npm:

```sh
mkdir hello-pdf && cd hello-pdf
npm init -y && npm pkg set type=module
npm install @laminapdf/sdk@0.2.0
```

The install names the version these docs describe, so the code below matches the package you get. `@laminapdf/sdk` brings its WebAssembly modules, `@laminapdf/wasm`, with it. [Install](../../pages/install.md) covers other package managers and Rust.

## Write the page

Save this app as `hello.ts`. It loads Lamina PDF, builds the page and writes it to `hello.pdf`:

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

On the docs site, the picture with the code is the page this app makes. To try a change before you set anything up, press **Edit and run** under the picture: your browser runs the code and shows the page it makes in place of the picture. Your browser can't write `hello.pdf` to your disk, so **Download PDF** saves it instead.

## Run it

```sh
node hello.ts
```

Node runs the TypeScript file as it is. Open `hello.pdf` in any PDF viewer: one page, with "Hello, world!" near its top-left corner.

## What happened

`init({ text: true })` loaded Lamina PDF's text module, the one that draws fonts; after it, every call is synchronous. The builder made an A4 page, drew the text on it and finished into a document, `doc.save()` turned that document into bytes, and `save` wrote them to `hello.pdf`. `using` freed the builder and the document as soon as the function ended.

Next, see [how Lamina PDF thinks about a PDF](../../pages/concepts/how-lamina-thinks.md) and [where things go on a page](../../pages/concepts/pages-and-coordinates.md). To use laminapdf somewhere other than Node, pick [your runtime](../../pages/quick-start/browser.md).
