// A Worker that generates a PDF per request: `/` a one-page letter for `?name=` (a 400 with the
// error as JSON for a name the standard font can't draw), `/report` the 100-page report with two
// embedded fonts (examples/report.ts). The fonts are this repository's fixtures; in your own
// Worker, bundle your font files the same way (wrangler.toml `rules`).
import { Builder, Font, init, isLaminaPdfError, type LaminaPdfError } from "@laminapdf/sdk";
import notoSans from "../../../../../fixtures/fonts/NotoSans-Regular.ttf";
import notoSansSC from "../../../../../fixtures/fonts/NotoSansSC-Regular.otf";
import { report } from "../../report.js";

/** The longest `?name=` the letter takes. */
const MAX_NAME = 100;

/**
 * A one-page letter to `name`, in a standard 14 font (nothing embedded, so only the characters
 * of its Latin character set: anything else throws `missing-glyph`).
 */
function letter(name: string): Uint8Array {
  const b = new Builder({ producer: "lamina Worker example" });
  try {
    const page = b.page("A4");
    page.surface().text(`Hello, ${name}!`, { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
    page.finish();
    return saveAndFree(b);
  } finally {
    // A no-op after finish(); when drawing threw, it frees the builder's WebAssembly side now
    // rather than whenever the garbage collector gets to it (an isolate serves many requests).
    b.free();
  }
}

/** Finishes `b`, saves the document and frees it (the bytes are a copy in JS memory). */
function saveAndFree(b: Builder): Uint8Array {
  const doc = b.finish();
  try {
    return doc.save();
  } finally {
    doc.free();
  }
}

const pdf = (bytes: Uint8Array): Response =>
  new Response(bytes as Uint8Array<ArrayBuffer>, { headers: { "content-type": "application/pdf" } });

/** Whether `e` is an error the name caused (see the `/` route). */
function isNameError(e: unknown): e is LaminaPdfError {
  return isLaminaPdfError(e) && (e.kind === "input" || e.kind === "limit");
}

/** A 400 with the error's ADR 0026 object as JSON. */
const badRequest = (body: unknown): Response => Response.json(body, { status: 400 });

// A default export because that is how a Worker module exposes its handlers (the named-exports
// rule is for package code).
export default {
  async fetch(request: Request): Promise<Response> {
    // Loads both modules once per isolate; later requests return at once.
    await init({ text: true });
    const url = new URL(request.url);
    switch (url.pathname) {
      case "/": {
        const name = url.searchParams.get("name") ?? "world";
        if (name.length > MAX_NAME) return badRequest({ message: `name is longer than ${MAX_NAME} characters` });
        try {
          return pdf(letter(name));
        } catch (e) {
          // A name Helvetica can't draw is the caller's input, not a server fault: a script it
          // lacks (`missing-glyph`, kind `input`: e.g. Cyrillic or Chinese) or more distinct
          // non-WinAnsi Latin letters than its 37 spare codes (`encoding-full`, kind `limit`).
          // Embed a font that covers them to accept them.
          if (isNameError(e)) return badRequest(e.toJSON());
          throw e;
        }
      }
      case "/report":
        // report() frees its builder and document itself.
        return pdf(report(new Uint8Array(notoSans), new Uint8Array(notoSansSC)));
      default:
        return new Response("not found", { status: 404 });
    }
  },
};
