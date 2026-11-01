// Builds a one-page PDF that says "Hello, world!" in the browser and offers it as a download.
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** Offers the PDF to the reader as hello.pdf. */
function download(pdf: Uint8Array): void {
  const url = URL.createObjectURL(new Blob([pdf.slice()], { type: "application/pdf" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = "hello.pdf";
  link.click();
  URL.revokeObjectURL(url);
}

/** Builds the page and hands its bytes to `save`, which offers a download unless you pass your own. */
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = download,
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
