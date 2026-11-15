// Switches of look (theme, accent, page appearance, density, direction) apply without transitions
// (design language §7.0, motion plan §3, 5): the root carries `data-motion-suppressed` from the
// switch until the frame after the switch draws, and base.css turns every transition off under
// it, so nothing fades from the old colours to the new.

/**
 * Turns transitions off on `root` (the document element when left out) around a switch of look
 * made now, until two animation frames have passed: the switch's styles apply in the first, and
 * transitions may start from the second. Calling it again before then extends the window. Without
 * a document or a window nothing draws, so it does nothing.
 */
export function suppressMotion(root?: HTMLElement): void {
  const target = root ?? (typeof document === "undefined" ? undefined : document.documentElement);
  const view = target?.ownerDocument.defaultView;
  if (target === undefined || view == null) return;
  target.setAttribute("data-motion-suppressed", "");
  const token = Symbol();
  pending.set(target, token);
  view.requestAnimationFrame(() =>
    view.requestAnimationFrame(() => {
      if (pending.get(target) !== token) return;
      pending.delete(target);
      target.removeAttribute("data-motion-suppressed");
    }),
  );
}

const pending = new WeakMap<HTMLElement, symbol>();
