// A browser app using @laminapdf/sdk, bundled by test/vite.test.ts.
import { Builder, Document, Font, init, loadText } from "@laminapdf/sdk";

/** Opens `bytes`, then builds a page of text: both modules, loaded the way a page would. */
export async function run(bytes: Uint8Array): Promise<readonly [number, number]> {
  await init();
  const pages = Document.open(bytes).pageCount;
  await loadText();
  const b = new Builder();
  const page = b.page("A6");
  page.surface().text("Hi", { font: Font.standard("Helvetica"), size: 9, at: [9, 20] });
  page.finish();
  return [pages, Document.open(b.finish().save()).pageCount];
}
