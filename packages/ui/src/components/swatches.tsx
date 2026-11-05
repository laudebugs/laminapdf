// Colour swatches (§7.12): round buttons named by colour, pressed while chosen. Colours are tokens
// only, so a swatch can't show a colour the design language doesn't name.

import type { TokenName } from "@laminapdf/tokens";
import type { CSSProperties, ReactNode } from "react";
import { ToggleButton } from "react-aria-components";
import { chooseOnPress, PressedGroup } from "./pressed-group.js";

/** One swatch: its id, its accessible name and the token it shows. */
export interface Swatch<Id extends string> {
  /** The id `onChange` receives. */
  id: Id;
  /** The colour's name ("Yellow"): the button's accessible name. */
  label: string;
  /** The colour, as a token: `annotation-yellow`, `content-line-red`. */
  color: TokenName;
}

/** Props of `SwatchGroup`. */
export interface SwatchGroupProps<Id extends string> {
  /** The swatches, in order. */
  swatches: readonly Swatch<Id>[];
  /** The chosen swatch. */
  value: Id;
  /** Called with a swatch when the user chooses it. */
  onChange: (id: Id) => void;
  /** The visible label before the row ("Colour"), which also names the group. */
  label: string;
  /** 24 px in a secondary toolbar (the default), 28 px in the inspector. On touch, 26 px in a 44 px target. */
  size?: 24 | 28;
}

/** A row of colour swatches; the chosen one has a 2 px ring in the text colour. */
export function SwatchGroup<Id extends string>({
  swatches,
  value,
  onChange,
  label,
  size = 24,
}: SwatchGroupProps<Id>): ReactNode {
  return (
    <PressedGroup label={label} className="paper-swatch-group" data-size={size}>
      <span className="paper-swatch-group-label" aria-hidden="true">
        {label}
      </span>
      {swatches.map((swatch) => (
        <ToggleButton
          key={swatch.id}
          aria-label={swatch.label}
          className="paper-swatch"
          isSelected={swatch.id === value}
          onChange={chooseOnPress(swatch.id, onChange)}
          style={{ "--swatch": `var(--paper-${swatch.color})` } as CSSProperties}
        >
          <span className="paper-swatch-dot" />
        </ToggleButton>
      ))}
    </PressedGroup>
  );
}
