// React Aria makes the page behind a modal inert but leaves `aria-modal` off the dialog, and drops
// it as a prop; the design language asks for it, so it is set on the element.

/** A ref callback that marks a modal dialog `aria-modal="true"`. */
export function markModal(element: HTMLElement | null): void {
  element?.setAttribute("aria-modal", "true");
}
