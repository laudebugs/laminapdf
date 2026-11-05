// @laminapdf/tokens: the Paper design language's values as typed constants. The same values are
// CSS custom properties in `@laminapdf/tokens/tokens.css`; both are generated from tokens.json.

import type { ANNOTATION_COLOURS, PAGE_APPEARANCES, THEMES, TokenName } from "./generated/tokens.js";

export {
  ANNOTATION_COLOURS,
  BREAKPOINTS,
  COMPONENT,
  CONTENT_COLOURS,
  FONTS,
  MOTION,
  PAGE_APPEARANCES,
  RADIUS,
  type SemanticColour,
  SPACE,
  THEMES,
  type TokenName,
  TYPE,
  Z_ORDER,
} from "./generated/tokens.js";

/** A UI theme: `data-theme` on the root (no attribute follows the system's light or dark). */
export type ThemeName = keyof typeof THEMES;

/** An accent: `data-accent` on the same element as `data-theme`. High contrast ignores it. */
export type AccentName = keyof (typeof THEMES)["light"];

/** How pages are drawn, apart from the UI theme: `data-page-appearance` on the canvas. */
export type PageAppearance = keyof typeof PAGE_APPEARANCES;

/** A highlight and annotation colour, the same in every theme and appearance. */
export type AnnotationColour = keyof typeof ANNOTATION_COLOURS;

/** The CSS reference to a token: `cssVar("surface")` is `var(--paper-surface)`. */
export function cssVar<N extends TokenName>(name: N): `var(--paper-${N})` {
  return `var(--paper-${name})`;
}
