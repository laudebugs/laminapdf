// Which parts of the viewer a selection may take in: only the text layer's spans. Each element
// that holds nothing to select sets `none` on itself; no ancestor of the spans may, since Firefox
// stops a mouse drag from selecting under a `user-select: none` ancestor once a text field on
// the page has had the focus (docs/bugs/firefox-selection-after-a-field).

/** Sets `element`'s `user-select`, with WebKit's prefix (Safari before 26 reads only that). */
export function setUserSelect(element: HTMLElement, value: "none" | "text"): void {
  element.style.setProperty("user-select", value);
  element.style.setProperty("-webkit-user-select", value);
}
