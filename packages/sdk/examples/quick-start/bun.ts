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
