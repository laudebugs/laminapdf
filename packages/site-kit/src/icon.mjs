// The Paper icons on the sites: each site imports an icon's file from @laminapdf/icons/svg/ with
// Vite's `?raw` and passes it here at build time, so a page carries the SVG and no script.

/**
 * A Paper icon as decorative HTML at 16 px: given a class, hidden from assistive technology and out
 * of the tab order. The control it sits in carries the name.
 *
 * @param {string} svg One file of @laminapdf/icons/svg/, as text.
 * @param {string} className
 * @returns {string}
 */
export function decorativeIcon(svg, className) {
  return svg.replace("<svg ", `<svg class="${className}" width="16" height="16" aria-hidden="true" focusable="false" `);
}
