// Playwright's WebKit reports every native <select> under `style-src 'self'` as an inline style it
// refused: the engine's own styles for the control, since a page holding only a bare <select> does
// the same (issue #492). Each select gives one console error, REFUSED_STYLE, and one
// `securitypolicyviolation` for `style-src-attr` aimed at the select.
//
// These helpers take out exactly one of each per select on the page, in WebKit only, so a real
// inline style still fails a test. The web reader's tests handle the same report another way
// (apps/web/e2e/reader.ts): their selects are made by script and gone before the check, so they
// match the report to the script line that made the select instead of counting.

/** WebKit's console message for a stylesheet or style attribute the CSP refused. */
export const REFUSED_STYLE =
  "Refused to apply a stylesheet because its hash, its nonce, or 'unsafe-inline' does not appear in the style-src directive of the Content Security Policy.";

/** A `securitypolicyviolation` as a test records it: its directive and its target's node name. */
export interface CspReport {
  readonly directive: string;
  readonly target: string;
}

/** `items` less the first `count` that `isQuirk` matches. */
function withoutFirst<T>(items: readonly T[], count: number, isQuirk: (item: T) => boolean): T[] {
  let left = count;
  return items.filter((item) => {
    if (left === 0 || !isQuirk(item)) return true;
    left -= 1;
    return false;
  });
}

/** `errors`, a page's console errors, less one {@link REFUSED_STYLE} per select when `browser` is WebKit. */
export function withoutSelectErrors(browser: string, errors: readonly string[], selects: number): string[] {
  return withoutFirst(errors, browser === "webkit" ? selects : 0, (text) => text === REFUSED_STYLE);
}

/** `reports` less one `style-src-attr` report aimed at a SELECT per select when `browser` is WebKit. */
export function withoutSelectReports<T extends CspReport>(
  browser: string,
  reports: readonly T[],
  selects: number,
): T[] {
  return withoutFirst(
    reports,
    browser === "webkit" ? selects : 0,
    (r) => r.directive === "style-src-attr" && r.target === "SELECT",
  );
}
