// `cargo xtask verify-install`'s check, copied next to the quick start in each temporary
// project: reopens the quick start's bytes and returns one `LAMINA_PDF_VERIFY {json}` line with their
// SHA-256, length and page count, which xtask compares with the Rust golden (hello.pdf).
import { Document } from "@laminapdf/sdk";

/** The `LAMINA_PDF_VERIFY` line for `bytes`. */
export async function report(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>));
  const sha256 = Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
  const doc = Document.open(bytes);
  try {
    return `LAMINA_PDF_VERIFY ${JSON.stringify({ sha256, length: bytes.length, pages: doc.pageCount })}`;
  } finally {
    doc.free();
  }
}
