// A browser app using @laminapdf/sdk, bundled by test/vite.test.ts: the browser quick start, and
// both modules loaded the way a page would.
import { Builder, Document, Font, init, loadText } from "@laminapdf/sdk";
import { hello } from "../../examples/quick-start/browser.js";
import { HELLO_OPTIONS } from "../suite/examples.js";

/** The browser quick start's bytes, built with the hello golden's options. */
export async function quickStart(): Promise<Uint8Array | undefined> {
  let saved: Uint8Array | undefined;
  await hello((pdf) => {
    saved = pdf;
  }, HELLO_OPTIONS);
  return saved;
}

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
