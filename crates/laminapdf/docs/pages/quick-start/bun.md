---
title: "Bun"
description: "Use @laminapdf/sdk in Bun."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/quick-start/bun/"
---

# Bun

Use @laminapdf/sdk in Bun.

Bun loads `@laminapdf/sdk` the way Node does: it reads the `.wasm` files from the `@laminapdf/wasm` package. Call `await init({ text: true })` once; after that, every call is synchronous.

This app builds one A4 page that says "Hello, world!" and writes it to `hello.pdf` with `Bun.write`:

**TypeScript**

`packages/sdk/examples/quick-start/bun.ts`:

```ts
// Builds a one-page PDF that says "Hello, world!" and writes it to hello.pdf, in Bun.
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** The space between the page's edge and the text, in points (72 points = 1 inch). */
const MARGIN = 72;
/** The text's size in points; its baseline sits this far below the margin. */
const FONT_SIZE = 24;

/** Builds the page and hands its bytes to `save`, which writes hello.pdf unless you pass another `save` function. */
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = async (pdf) => {
    await Bun.write("hello.pdf", pdf);
  },
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

// `bun hello.ts` runs it; imported, it only exports `hello`.
if (import.meta.main) await hello();
```

**Result:** [hello.pdf](https://docs.laminapdf.com/examples/pdf/hello.pdf)

Save it as `hello.ts` in a project with `@laminapdf/sdk` [installed](../../pages/install.md), and run it:

```sh
bun hello.ts
```

Lamina PDF's tests run this app in Bun and check that it builds the reference PDF byte for byte.

Next: draw text with [fonts](../../pages/guides/fonts-and-text.md), or [open and edit](../../pages/guides/edit-a-pdf.md) a file.
