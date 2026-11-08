// The tokens route: the theme's colours with the contrast pairs tokens.json requires of them, then
// the type scale, spacing, radii and shadows. Every swatch is drawn with the CSS variable, so the
// theme on the root decides what it shows.

import { ICON_SIZES } from "@laminapdf/icons";
import { RADIUS, SPACE, THEMES, TYPE } from "@laminapdf/tokens";
import tokens from "@laminapdf/tokens/tokens.json";
import type { ReactNode } from "react";

/** A contrast pair as tokens.json writes it: `on` a background, at least `min`. */
interface Pair {
  on: string;
  min: number;
  kind?: string;
  /** The opaque colour under a translucent `on` (hover and pressed). */
  over?: string;
  /** Why the pair needn't meet `min`, such as a disabled control (WCAG 1.4.3). */
  exempt?: string;
}

/** `theme`'s contrast pairs per colour: each theme names its own. */
function pairs(theme: string): [string, Pair[]][] {
  const themes = (tokens as { theme: Record<string, Record<string, unknown> | string> }).theme;
  const colours = themes[theme];
  if (colours === undefined || typeof colours === "string") return [];
  return Object.entries(colours).flatMap(([name, value]) => {
    const extension = (value as { $extensions?: { "dev.lamina"?: { contrast?: Pair[] } } }).$extensions;
    const list = extension?.["dev.lamina"]?.contrast;
    return list === undefined ? [] : [[name, list] as [string, Pair[]]];
  });
}

const variable = (name: string) => `var(--paper-${name})`;
const bare = (ref: string) => ref.replace(/[{}]/g, "");

/** One contrast pair: `name` drawn on its background, with what the pair requires. */
function PairItem({ name, pair }: { name: string; pair: Pair }): ReactNode {
  const boundary = pair.kind === "boundary";
  return (
    <li>
      <span className="g-pair-base" style={{ background: variable(bare(pair.over ?? pair.on)) }}>
        <span className="g-pair-sample" style={{ background: variable(bare(pair.on)) }}>
          {boundary ? (
            <span className="g-boundary" style={{ borderColor: variable(name) }} aria-hidden="true" />
          ) : (
            // An exempt pair stands for a disabled control's text, and is marked as one.
            <span
              className={pair.min < 4.5 ? "g-large" : undefined}
              style={{ color: variable(name) }}
              aria-disabled={pair.exempt === undefined ? undefined : true}
            >
              {name}
            </span>
          )}
        </span>
      </span>
      <small>
        {name} on {bare(pair.on)}
        {pair.over === undefined ? "" : ` over ${bare(pair.over)}`}, at least {pair.min}:1
        {boundary ? " (a line, not text)" : ""}
        {pair.exempt === undefined ? "" : ` (exempt: ${pair.exempt})`}
      </small>
    </li>
  );
}

/** The tokens route in `theme` (the UI theme the root has). */
export function Tokens({ theme }: { theme: string }): ReactNode {
  const colours = Object.keys(THEMES.light.teal);
  return (
    <>
      <section aria-labelledby="t-colours">
        <h2 id="t-colours">Colours</h2>
        <ul className="g-swatches">
          {colours.map((name) => (
            <li key={name}>
              <span className="g-swatch" style={{ background: variable(name) }} />
              <code>{name}</code>
            </li>
          ))}
        </ul>
      </section>
      <section aria-labelledby="t-pairs">
        <h2 id="t-pairs">Contrast pairs</h2>
        <ul className="g-pairs">
          {pairs(theme).flatMap(([name, list]) =>
            list.map((pair) => <PairItem key={`${name}-${pair.on}`} name={name} pair={pair} />),
          )}
        </ul>
      </section>
      <section aria-labelledby="t-type">
        <h2 id="t-type">Type scale</h2>
        {Object.keys(TYPE).map((name) => (
          <p key={name} className={`paper-type-${name}`} style={{ font: `var(--paper-type-${name})` }}>
            {name}: Find in document
          </p>
        ))}
      </section>
      <section aria-labelledby="t-space">
        <h2 id="t-space">Spacing, radii and shadows</h2>
        <ul className="g-scale">
          {Object.keys(SPACE).map((name) => (
            <li key={name}>
              <span className="g-bar" style={{ width: `var(--paper-space-${name})` }} />
              <code>space-{name}</code>
            </li>
          ))}
        </ul>
        <ul className="g-scale g-row">
          {Object.keys(RADIUS).map((name) => (
            <li key={name}>
              <span className="g-box" style={{ borderRadius: `var(--paper-radius-${name})` }} />
              <code>radius-{name}</code>
            </li>
          ))}
        </ul>
        <ul className="g-scale g-row">
          {["page", "thumbnail", "float", "sheet", "overlay"].map((name) => (
            <li key={name}>
              <span className="g-box" style={{ boxShadow: `var(--paper-shadow-${name})` }} />
              <code>shadow-{name}</code>
            </li>
          ))}
        </ul>
        <p className="g-note">Icon sizes: {ICON_SIZES.join(", ")} px.</p>
      </section>
    </>
  );
}
