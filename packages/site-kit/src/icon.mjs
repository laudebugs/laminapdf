// The Paper icons on the sites: each site imports an icon's file from packages/icons/svg/ with
// Vite's `?raw` and passes it here at build time, so a page carries the SVG and no script.

/**
 * A Paper icon as decorative HTML: sized, given a class, hidden from assistive technology and out of
 * the tab order. The control it sits in carries the name.
 *
 * @param {string} svg One file of packages/icons/svg/, as text.
 * @param {string} className
 * @param {number} [size] In CSS pixels.
 * @returns {string}
 */
export function decorativeIcon(svg, className, size = 16) {
  return svg.replace(
    "<svg ",
    `<svg class="${className}" width="${size}" height="${size}" aria-hidden="true" focusable="false" `,
  );
}
