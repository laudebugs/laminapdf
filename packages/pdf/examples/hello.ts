// The smallest document: one A4 page with a line of text in a standard 14 font. The TS twin of
// crates/lamina/examples/hello.rs: the same bytes (fixtures/golden/examples/hello.pdf).
import { Builder, Font } from "@lamina/pdf";

/** The document's bytes: the same on every run and every platform. Needs `init({ text: true })`. */
export function hello(): Uint8Array {
  // Everything that would come from the environment (file ID, dates) is given.
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-hello!"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
  });
  const page = b.page("A4");
  // Page space: points from the top-left corner, y down; `at[1]` is the text's baseline.
  page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
  page.finish();
  // Free the document at once (the saved bytes are a copy): a Worker or a timing loop calling
  // this many times shouldn't wait for the garbage collector to release WebAssembly memory.
  const doc = b.finish();
  try {
    return doc.save();
  } finally {
    doc.free();
  }
}
