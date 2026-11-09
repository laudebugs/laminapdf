// What a network request was for, decided in one place: the status bar's count, the network
// panel and the e2e tests that hold them to the browser's own request log all call
// `classifyRequest` (ADR 0011, R13).
//
// - App files: anything from the reader's own origin. It serves static files only (the page,
//   hashed scripts, styles, fonts and WebAssembly modules, ADR 0011 §1), so none of it is
//   document traffic, whenever it loads (PDFium's module may load after a file is chosen).
// - Document traffic: requests to the origin of the open `#url=` document, which the document
//   worker fetches itself.
// - Unexpected: anything else. The reader never means to make such a request.

/** The group a request falls in. */
export type RequestGroup = "app" | "document" | "unexpected";

/** What a request was for, for the panel's purpose line. */
export type RequestPurpose = "page" | "script" | "style" | "font" | "image" | "webassembly" | "document" | "other";

/** What a request is classified against. */
export interface ClassifyContext {
  /** The reader's own origin (`location.origin`). */
  readonly appOrigin: string;
  /** The open remote document's address, when one is open. */
  readonly documentUrl?: string | undefined;
}

/** `url`'s origin, or undefined for a URL that has none (`data:`, `blob:`, a bad URL). */
export function originOf(url: string): string | undefined {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:" ? parsed.origin : undefined;
  } catch {
    return undefined;
  }
}

/** Which group a request to `url` falls in. */
export function classifyRequest(url: string, context: ClassifyContext): RequestGroup {
  const origin = originOf(url);
  if (context.documentUrl !== undefined && origin !== undefined && origin === originOf(context.documentUrl)) {
    return "document";
  }
  if (origin === context.appOrigin) return "app";
  return "unexpected";
}

/** What a request was for, from its path and what made it. */
export function purposeOf(url: string, initiator: string, group: RequestGroup): RequestPurpose {
  if (group === "document") return "document";
  const path = (() => {
    try {
      return new URL(url).pathname.toLowerCase();
    } catch {
      return "";
    }
  })();
  if (initiator === "navigation") return "page";
  // The file's own extension first: a font loaded by a stylesheet has initiator `css`.
  if (path.endsWith(".wasm")) return "webassembly";
  if (path.endsWith(".js") || path.endsWith(".mjs")) return "script";
  if (path.endsWith(".css")) return "style";
  if (/\.(woff2?|ttf|otf)$/.test(path)) return "font";
  if (/\.(png|jpe?g|gif|svg|webp|avif|ico)$/.test(path)) return "image";
  if (path === "/" || path.endsWith(".html")) return "page";
  if (initiator === "script") return "script";
  if (initiator === "img") return "image";
  if (initiator === "css" || initiator === "link") return "style";
  return "other";
}
