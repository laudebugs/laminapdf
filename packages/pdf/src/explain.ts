// `@lamina/pdf/explain` (ADR 0026 §6): every code's explain page, kind and hint, for tools and
// agents that want to tell a user what a code means and what to do. A separate subpath, so the
// ~80 KB of page text and the tables stay out of the main bundle (and out of the WebAssembly).
import { type ErrorCode, HINTS, type WarningCode } from "./generated/codes.js";
import { PAGES } from "./generated/explain.js";

export type { ErrorCode, ErrorKind, WarningCode } from "./generated/codes.js";
export { DOCS_URL, docsUrl, ERROR_CODES, HINTS, KINDS, WARNING_CODES } from "./generated/codes.js";

/** Whether `code` is one of lamina's error or warning codes. */
export function isCode(code: string): code is ErrorCode | WarningCode {
  return Object.hasOwn(HINTS, code);
}

/**
 * The explain page of `code` as Markdown (`docs/errors/<code>.md`: what happened, why, how to fix,
 * an example and the `data` fields), or `undefined` for a string that is not a code. The same
 * text as `docsUrl` and `lamina::explain` in Rust.
 *
 * @param code - an error or warning code, such as `LaminaError.code`.
 * @example
 * ```ts
 * import { explain } from "@lamina/pdf/explain";
 * console.log(explain("wrong-password"));
 * ```
 */
export function explain(code: ErrorCode | WarningCode): string;
export function explain(code: string): string | undefined;
export function explain(code: string): string | undefined {
  return isCode(code) ? PAGES[code] : undefined;
}
