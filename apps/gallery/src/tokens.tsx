// The tokens route: the theme's colours with the contrast pairs tokens.json requires of them, then
// the type scale, spacing, radii and shadows. Every swatch is drawn with the CSS variable, so the
// theme on the root decides what it shows.

import { ICON_SIZES } from "@laminapdf/icons";
import { RADIUS, SPACE, THEMES, TYPE } from "@laminapdf/tokens";
import type { ReactNode } from "react";
import tokens from "../../../packages/tokens/tokens.json";

/** A contrast pair as tokens.json writes it: `on` a background, at least `min`. */
interface Pair {
  on: string;
  min: number;
  kind?: string;
}

/** The light theme's contrast pairs per colour (the other themes name the same pairs). */
function pairs(): [string, Pair[]][] {
  const light = (tokens as { theme: { light: Record<string, unknown> } }).theme.light;
  return Object.entries(light).flatMap(([name, value]) => {
    const extension = (value as { $extensions?: { "dev.lamina"?: { contrast?: Pair[] } } }).$extensions;
    const list = extension?.["dev.lamina"]?.contrast;
    return list === undefined ? [] : [[name, list] as [string, Pair[]]];
  });
}

const variable = (name: string) => `var(--paper-${name})`;
const bare = (ref: string) => ref.replace(/[{}]/g, "");

export function Tokens(): ReactNode {
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
          {pairs().flatMap(([name, list]) =>
            list.map((pair) => (
              <li key={`${name}-${pair.on}`} style={{ background: variable(bare(pair.on)) }}>
                <span
                  style={{ color: variable(name), borderColor: variable(name) }}
                  className={pair.kind === "boundary" ? "g-boundary" : undefined}
                >
                  {name}
                </span>
                <small>
                  on {bare(pair.on)}, at least {pair.min}:1
                </small>
              </li>
            )),
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
