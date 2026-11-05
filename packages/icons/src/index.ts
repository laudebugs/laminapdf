// @laminapdf/icons: the Paper design language's stroke icons (§5) as markup, with no framework, so
// the React chrome, the framework-free viewer and native menus draw the same glyphs. The React
// component is the separate `@laminapdf/icons/react` entry.

import { ICONS, type IconName } from "./generated/icons.js";

export { ICONS, type IconName } from "./generated/icons.js";

/** The sizes the design language renders icons at, in CSS px (32 for empty states). */
export const ICON_SIZES = [14, 15, 16, 18, 20, 22, 32] as const;

/** A size the design language renders icons at. */
export type IconSize = (typeof ICON_SIZES)[number];

/** Options for `iconSvg`. */
export interface IconSvgOptions {
  /** Width and height in CSS px; 16 when left out. */
  size?: IconSize;
  /**
   * An accessible name, for an icon that stands alone (`role="img"`). Leave it out inside a
   * labelled button: the icon is then hidden from assistive technology.
   */
  title?: string;
}

const escapeAttribute = (text: string) =>
  text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] ?? c);

/** A complete `<svg>` for the icon, stroked in `currentColor`, decorative unless given a `title`. */
export function iconSvg(name: IconName, options: IconSvgOptions = {}): string {
  const size = options.size ?? 16;
  const a11y =
    options.title === undefined
      ? 'aria-hidden="true" focusable="false"'
      : `role="img" aria-label="${escapeAttribute(accessibleName(options.title))}"`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" ${a11y}>${ICONS[name]}</svg>`;
}

/**
 * The attributes an icon-only button needs: its accessible name, a verb phrase such as "Find in
 * document" (design language §5). Throws on an empty label, so a button can't ship without one.
 */
export function iconButtonProps(label: string): { "aria-label": string } {
  return { "aria-label": accessibleName(label) };
}

function accessibleName(label: string): string {
  const name = label.trim();
  if (name === "") throw new Error("an icon that stands alone needs a non-empty accessible name");
  return name;
}
