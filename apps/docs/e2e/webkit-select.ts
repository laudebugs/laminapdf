// Playwright's WebKit reports every native <select> under `style-src 'self'` as an inline style it
// refused: the engine's own styles for the control (a page holding only a bare <select> does the
// same, issue #492). The docs' theme switch is a <select>, in the header and in the phone menu, so
// since `'unsafe-inline'` left style-src (#470) each page logs the refusal twice in WebKit. These
// helpers leave out exactly those reports, one per <select>, and keep everything else, so a real
// inline style still fails the suite. apps/web/e2e/reader.ts does the same for the web reader.

import type { Page } from "playwright";

const BROWSER = process.env.LAMINA_PDF_E2E_BROWSER ?? "chromium";

/** WebKit's console message for a stylesheet or style attribute the CSP refused. */
const REFUSED_STYLE =
  "Refused to apply a stylesheet because its hash, its nonce, or 'unsafe-inline' does not appear in the style-src directive of the Content Security Policy.";

/**
 * `errors`, the page's console errors, less one {@link REFUSED_STYLE} for each <select> on the page,
 * in WebKit only. It counts the selects through Playwright, so it works with the page's JavaScript off.
 */
export async function withoutSelectReports(page: Page, errors: readonly string[]): Promise<string[]> {
  if (BROWSER !== "webkit") return [...errors];
  let selects = await page.locator("select").count();
  return errors.filter((text) => {
    if (text !== REFUSED_STYLE || selects === 0) return true;
    selects -= 1;
    return false;
  });
}

/** Whether a `securitypolicyviolation` aimed at `target` (its node name) is WebKit's <select> report. */
export const isSelectReport = (directive: string, target: string): boolean =>
  BROWSER === "webkit" && directive === "style-src-attr" && target === "SELECT";
