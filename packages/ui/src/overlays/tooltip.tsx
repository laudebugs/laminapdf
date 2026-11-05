// Tooltips (§7.27): a short label and the shortcut chip, 6 px from the control, shown after 500 ms
// of hover and at once on keyboard focus, dismissed by Escape. Never the only place for something:
// the control's own name says what it does.

import { COMPONENT, MOTION } from "@laminapdf/tokens";
import type { ReactElement, ReactNode } from "react";
import { Tooltip as AriaTooltip, type Placement, TooltipTrigger } from "react-aria-components";
import { type KeyPlatform, ShortcutChip, type ShortcutKeys } from "../components/shortcut-chip.js";

/** Props of `Tooltip`. */
export interface TooltipProps {
  /** The control it describes: a `Button`, `IconButton` or another focusable component. */
  children: ReactElement;
  /** The text: usually the control's name. */
  text: string;
  /** The control's shortcut, shown as a chip. */
  shortcut?: ShortcutKeys;
  /** How the chip is written; needed with `shortcut`. */
  platform?: KeyPlatform;
  /** Where it sits; below when left out. */
  placement?: Placement;
}

/** A tooltip on `children`. */
export function Tooltip({
  children,
  text,
  shortcut,
  platform = "other",
  placement = "bottom",
}: TooltipProps): ReactNode {
  return (
    <TooltipTrigger delay={MOTION["tooltip-delay"]}>
      {children}
      <AriaTooltip className="paper-tooltip" placement={placement} offset={COMPONENT["tooltip-offset"]}>
        {text}
        {shortcut === undefined ? null : <ShortcutChip keys={shortcut} platform={platform} />}
      </AriaTooltip>
    </TooltipTrigger>
  );
}
