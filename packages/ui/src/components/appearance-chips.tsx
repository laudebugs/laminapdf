// Page-appearance chips (§7.25): one chip per appearance, each previewing "Aa" in that
// appearance's paper and ink, pressed while chosen.

import type { PageAppearance } from "@laminapdf/tokens";
import type { ReactNode } from "react";
import { ToggleButton } from "react-aria-components";

const APPEARANCES: readonly PageAppearance[] = ["original", "sepia", "dark", "high-contrast"];

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
    // biome-ignore lint/a11y/useSemanticElements: a group of pressed buttons, as §7.25 specifies; <fieldset> holds form controls.
    <div role="group" aria-label={label} className="paper-appearance-chips">
      {APPEARANCES.map((appearance) => (
        <ToggleButton
          key={appearance}
          className="paper-appearance-chip"
          isSelected={appearance === value}
          onChange={(selected) => {
            if (selected) onChange(appearance);
          }}
        >
          <span className="paper-appearance-preview" data-page-appearance={appearance} aria-hidden="true">
            Aa
          </span>
          <span className="paper-appearance-label">{labels[appearance]}</span>
        </ToggleButton>
      ))}
    </div>
  );
}
