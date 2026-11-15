// What makes the desktop window behave as a window rather than a web page (motion plan §5): the
// webview's own context menu (Back, Reload, Inspect Element) never opens on the chrome, and the
// root carries `data-window-inactive` while another window has the focus, which app.css dims the
// bars by. The chrome is where app.css turns selection off, so the two never disagree: the
// document, the reader view, Document properties, the failure page and fields keep their menu,
// with Copy and Paste.

/** The page's own background (the root, the body, the app's root): no text there, and no menu. */
const BACKGROUND = new Set(["HTML", "BODY"]);

/**
 * Whether a right-click on `target` may show the webview's context menu: where its text can be
 * selected (app.css), as computed in `view`. `auto` takes its parent's, so the nearest element that
 * sets a value decides; the page's background never keeps the menu.
 */
export function keepsContextMenu(target: EventTarget | null, view: Window): boolean {
  if (!(target instanceof Element) || BACKGROUND.has(target.tagName) || target.id === "app") return false;
  for (let at: Element | null = target; at !== null; at = at.parentElement) {
    const style = view.getComputedStyle(at);
    const select = style.userSelect || style.getPropertyValue("-webkit-user-select");
    if (select === "none") return false;
    if (select !== "" && select !== "auto") return true;
  }
  return true;
}

/**
 * Starts both behaviours on `view`'s document.
 * @param view - The window (`window`); its document's root takes the attribute.
 * @returns A function that stops them.
 */
export function watchWindowState(view: Window): () => void {
  const root = view.document.documentElement;
  const menu = (event: MouseEvent) => {
    if (!keepsContextMenu(event.target, view)) event.preventDefault();
  };
  const show = () => {
    if (view.document.hasFocus()) root.removeAttribute("data-window-inactive");
    else root.setAttribute("data-window-inactive", "");
  };
  view.document.addEventListener("contextmenu", menu);
  view.addEventListener("focus", show);
  view.addEventListener("blur", show);
  show();
  return () => {
    view.document.removeEventListener("contextmenu", menu);
    view.removeEventListener("focus", show);
    view.removeEventListener("blur", show);
  };
}
