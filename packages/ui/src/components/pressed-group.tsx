// What the mode switcher, swatches and appearance chips share: a labelled group of pressed
// buttons where exactly one is chosen. Pressing the chosen one again changes nothing.

import type { HTMLAttributes, ReactNode } from "react";

/** A `role="group"` element named by `label`, holding pressed buttons. */
export function PressedGroup({
  label,
  children,
  ...rest
}: { label: string; children: ReactNode } & Omit<HTMLAttributes<HTMLDivElement>, "role" | "aria-label">): ReactNode {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a group of pressed buttons, as the design language specifies; <fieldset> holds form controls.
    <div {...rest} role="group" aria-label={label}>
      {children}
    </div>
  );
}

/** A ToggleButton's onChange that reports `id` when it becomes pressed and ignores unpressing. */
export function chooseOnPress<Id extends string>(id: Id, onChange: (id: Id) => void): (selected: boolean) => void {
  return (selected) => {
    if (selected) onChange(id);
  };
}
