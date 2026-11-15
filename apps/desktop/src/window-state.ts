// What makes the desktop window behave as a window rather than a web page (motion plan §5): the
// webview's own context menu (Back, Reload, Inspect Element) never opens on the chrome, and the
// root carries `data-window-inactive` while another window has the focus, which app.css dims the
// bars by. The document, the reader view's column and fields keep their context menu, with Copy
// and Paste.

/** Where the webview's own context menu still opens: text people select, copy or edit. */
const KEEPS_MENU = ".desktop-viewer, .paper-reader-column, input, textarea, [contenteditable]";

/**
 * Whether a right-click on `target` may show the webview's context menu.
 * @param target - The event's target.
 * @returns True inside the document, the reader view's column and fields.
 */
export function keepsContextMenu(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(KEEPS_MENU) !== null;
}

/**
 * Starts both behaviours on `view`'s document.
 * @param view - The window (`window`); its document's root takes the attribute.
 * @returns A function that stops them.
 */
export function watchWindowState(view: Window): () => void {
  const root = view.document.documentElement;
  const menu = (event: MouseEvent) => {
    if (!keepsContextMenu(event.target)) event.preventDefault();
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
