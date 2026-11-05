// Page-appearance chips (§7.25): one chip per appearance, each previewing "Aa" in that
// appearance's paper and ink, pressed while chosen.

import { PAGE_APPEARANCES, type PageAppearance } from "@laminapdf/tokens";
import type { ReactNode } from "react";
import { ToggleButton } from "react-aria-components";
import { chooseOnPress, PressedGroup } from "./pressed-group.js";

// In the tokens' order: Original, Sepia, Dark, High contrast.
const APPEARANCES = Object.keys(PAGE_APPEARANCES) as PageAppearance[];

/** Props of `PageAppearanceChips`. */
export interface PageAppearanceChipsProps {
  /** The chosen appearance. */
  value: PageAppearance;
  /** Called with an appearance when the user chooses it. */
  onChange: (appearance: PageAppearance) => void;
  /** Each appearance's visible name ("Original", "Sepia", "Dark", "High contrast"). */
  labels: Readonly<Record<PageAppearance, string>>;
  /** The group's accessible name ("Page appearance"). */
  "aria-label": string;
}

/** The four page appearances as chips in a grid; the chosen one is white with an accent border. */
export function PageAppearanceChips({
  value,
  onChange,
  labels,
  "aria-label": label,
}: PageAppearanceChipsProps): ReactNode {
  return (
    <PressedGroup label={label} className="paper-appearance-chips">
      {APPEARANCES.map((appearance) => (
        <ToggleButton
          key={appearance}
          className="paper-appearance-chip"
          isSelected={appearance === value}
          onChange={chooseOnPress(appearance, onChange)}
        >
          <span className="paper-appearance-preview" data-page-appearance={appearance} aria-hidden="true">
            Aa
          </span>
          <span className="paper-appearance-label">{labels[appearance]}</span>
        </ToggleButton>
      ))}
    </PressedGroup>
  );
}
