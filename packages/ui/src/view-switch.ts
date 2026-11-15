// Switching whole views (motion plan §4.3): the Reader toggle swaps the page view for the reading
// column and back inside a same-document view transition. The root carries `data-view-switch`
// while one runs, which names the reader's canvas `paper-view` (reader.css), so only the canvas
// animates and the chrome around it changes at once. Where view transitions are missing, or the
// page is hidden, the switch is instant.

import { flushSync } from "react-dom";

/** The newest transition: it owns the root's attribute, which an older one leaves alone when it ends. */
let newest: symbol | undefined;

/**
 * Runs `update`, a React state change that swaps one whole view for another, inside a view
 * transition: the browser draws the old view, React renders the new one at once (`flushSync`, so
 * the new view is in the DOM when the browser captures it), and the two cross-fade over
 * `duration-view`. Without `document.startViewTransition`, or with the page hidden, `update` runs
 * straight away.
 */
export function switchView(update: () => void): void {
  const doc = typeof document === "undefined" ? undefined : document;
  if (doc?.startViewTransition === undefined || doc.visibilityState === "hidden") {
    update();
    return;
  }
  const root = doc.documentElement;
  const token = Symbol();
  newest = token;
  root.setAttribute("data-view-switch", "");
  const transition = doc.startViewTransition(() => flushSync(update));
  // A skipped transition (a newer one started, or the page was hidden) still runs `update`.
  transition.ready.catch(() => undefined);
  void transition.finished
    .catch(() => undefined)
    .then(() => {
      if (newest !== token) return;
      newest = undefined;
      root.removeAttribute("data-view-switch");
    });
}
