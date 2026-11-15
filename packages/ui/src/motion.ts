// Switches of look (theme, accent, page appearance, density) apply without transitions (design
// language §7.0, motion plan §3, 5): for one frame around the switch, the root carries
// `data-motion-suppressed`, which base.css reads to turn every transition off, so nothing fades
// from the old colours to the new.

/**
 * Turns transitions off on `root` until the frame after the next, around a switch of look made
 * now. Calling it again before then extends the window.
 */
export function suppressMotion(root: HTMLElement = document.documentElement): void {
  // Without a window nothing draws, so nothing could transition: leave the root alone.
  const view = root.ownerDocument.defaultView;
  if (view === null) return;
  root.setAttribute("data-motion-suppressed", "");
  // Two frames: the switch's styles apply in the first, and transitions start from the second.
  const token = Symbol();
  pending.set(root, token);
  view.requestAnimationFrame(() =>
    view.requestAnimationFrame(() => {
      if (pending.get(root) !== token) return;
      pending.delete(root);
      root.removeAttribute("data-motion-suppressed");
    }),
  );
}

const pending = new WeakMap<HTMLElement, symbol>();
