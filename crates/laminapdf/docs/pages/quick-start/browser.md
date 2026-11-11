---
title: "Browser and bundlers"
description: "Load @laminapdf/sdk in a web page through Vite or another bundler."
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/quick-start/browser/"
---

# Browser and bundlers

Load @laminapdf/sdk in a web page through Vite or another bundler.

In a browser, `@laminapdf/sdk` finds its `.wasm` files with `new URL("…", import.meta.url)`. Your bundler emits them as assets next to your code, so they load from your own origin.

This app builds one A4 page that says "Hello, world!" and offers it to your user as `hello.pdf`:

**TypeScript**

`packages/sdk/examples/quick-start/browser.ts`:

```ts
// Builds a one-page PDF that says "Hello, world!" in the browser and offers it as a download.
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** The space between the page's edge and the text, in points (72 points = 1 inch). */
const MARGIN = 72;
/** The text's size in points; its baseline sits this far below the margin. */
const FONT_SIZE = 24;

/** Offers the PDF to your user as hello.pdf. */
function download(pdf: Uint8Array): void {
  // `slice()` copies the bytes into a plain ArrayBuffer, which is what a Blob accepts.
  const url = URL.createObjectURL(new Blob([pdf.slice()], { type: "application/pdf" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = "hello.pdf";
  link.click();
  // Revoke the URL after the click has been handled; revoking it at once can cancel the download.
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

/** Builds the page and hands its bytes to `save`, which offers a download unless you pass another `save` function. */
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = download,
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
```

**Result:** [hello.pdf](https://docs.laminapdf.com/examples/pdf/hello.pdf)

Import `hello` from your page's script and call it when your user asks for the file, for example from a button: `button.addEventListener("click", () => hello())`. Then start the page with your bundler, such as `npx vite`.

Lamina PDF's tests bundle this app with Vite, run it in Chromium, Firefox and WebKit, and check that it builds the reference PDF byte for byte.

- **Modules on demand:** a page that never draws text never downloads the text module; see [three modules in JavaScript](../../pages/concepts/how-lamina-thinks.md#three-modules-in-javascript).
- **Vite:** exclude the package from dependency pre-bundling (`optimizeDeps.exclude: ["@laminapdf/sdk"]` in `vite.config.ts`), so the `.wasm` files stay next to the JavaScript that loads them.
- **Content Security Policy:** compiling WebAssembly needs `script-src 'wasm-unsafe-eval'`. Nothing else is needed.

Next: draw text with [fonts](../../pages/guides/fonts-and-text.md), or [open and edit](../../pages/guides/edit-a-pdf.md) a file.
