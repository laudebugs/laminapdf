// The mode switcher (§7.2): a group of buttons with `aria-pressed`, not tabs, because a mode
// changes the tools rather than showing a panel. Each button is its own Tab stop, as the group
// role tells assistive technology to expect; arrow keys are left to the page.

import type { ReactNode } from "react";
import { ToggleButton } from "react-aria-components";

/** One mode: its id, used in `onChange`, and its visible name. */
export interface Mode<Id extends string> {
  /** The mode's id. */
  id: Id;
  /** The visible name ("Read", "Comment"). */
  label: string;
}

/** Props of `ModeSwitcher`. */
export interface ModeSwitcherProps<Id extends string> {
  /** The modes, in order. */
  modes: readonly Mode<Id>[];
  /** The current mode. */
  value: Id;
  /** Called with a mode when the user switches to it (never for the current one). */
  onChange: (mode: Id) => void;
  /** The group's accessible name; "Mode" in English. */
  "aria-label": string;
}

/** The top bar's mode switcher: the current mode bold with an accent underline. */
export function ModeSwitcher<Id extends string>({
  modes,
  value,
  onChange,
  "aria-label": label,
}: ModeSwitcherProps<Id>): ReactNode {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a group of pressed buttons, as §7.2 specifies; <fieldset> holds form controls.
    <div role="group" aria-label={label} className="paper-mode-switcher">
      {modes.map((mode) => (
        <ToggleButton
          key={mode.id}
          className="paper-mode"
          isSelected={mode.id === value}
          onChange={(selected) => {
            if (selected) onChange(mode.id);
          }}
        >
          {mode.label}
        </ToggleButton>
      ))}
    </div>
  );
}
