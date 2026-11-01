// Builds a one-page PDF that says "Hello, world!" and writes it to hello.pdf, in Node.
import { writeFile } from "node:fs/promises";
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** Builds the page and hands its bytes to `save`, which writes hello.pdf unless you pass your own. */
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = (pdf) => writeFile("hello.pdf", pdf),
  options: BuilderOptions = {},
): Promise<void> {
  await init({ text: true });
  using builder = new Builder(options);
  const page = builder.page("A4");
  // Points from the page's top-left corner; the second number is the text's baseline.
  page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
  page.finish();
  using doc = builder.finish();
  await save(doc.save());
}

// `node hello.ts` runs it (Node 24); imported, it only exports `hello`.
if (import.meta.main) await hello();
