// Builds a one-page PDF that says "Hello, world!" in the browser and offers it as a download.
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** The space between the page's edge and the text, in points (72 points = 1 inch). */
const MARGIN = 72;
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
  // Points from the page's top-left corner, y down; the text sits on a baseline one line below the margin.
  page.surface().text("Hello, world!", {
    font: Font.standard("Helvetica"),
    size: FONT_SIZE,
    at: [MARGIN, MARGIN + FONT_SIZE],
  });
  page.finish();
  using doc = builder.finish();
  await save(doc.save());
}
