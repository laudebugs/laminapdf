// The header's phone menu is a <details>, so it opens with no script. This adds what a menu is
// expected to do beyond that: Escape closes it and puts the focus back on its button, and a press
// outside closes it.

/**
 * Wires every `details.site-menu` under `root`.
 *
 * @param {ParentNode & Node} root
 */
export function wireMenus(root) {
  const menus = [...root.querySelectorAll("details.site-menu")];
  for (const menu of menus) {
    if (!(menu instanceof HTMLDetailsElement)) continue;
    menu.addEventListener("keydown", (event) => {
      if (event.key !== "Escape" || !menu.open) return;
      menu.open = false;
      menu.querySelector("summary")?.focus();
    });
  }
  root.addEventListener("pointerdown", (event) => {
    for (const menu of menus) {
      if (menu instanceof HTMLDetailsElement && menu.open && !menu.contains(/** @type {Node} */ (event.target))) {
        menu.open = false;
      }
    }
  });
}
