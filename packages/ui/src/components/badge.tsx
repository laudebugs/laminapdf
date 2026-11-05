// Badges and tags (§7.23): a short word on a tint in its status colour, with an optional icon, so
// the meaning never rests on colour alone. A tag is the same component naming a category.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";

/** A badge's status colour. */
export type BadgeTone = "neutral" | "accent" | "positive" | "warning" | "danger";

/** Props of `Badge`. */
export interface BadgeProps {
  /** The word or words: "Signature valid", "Approximate". Required, so colour never stands alone. */
  children: ReactNode;
  /** The status colour; `neutral` when left out. */
  tone?: BadgeTone;
  /** An icon before the words. */
  icon?: IconName;
}

/** A badge or tag: not interactive, so it sits inside a button when it needs to open something. */
export function Badge({ children, tone = "neutral", icon }: BadgeProps): ReactNode {
  return (
    <span className="paper-badge" data-tone={tone}>
      {/* badge.css sizes the icon to 12 px. */}
      {icon === undefined ? null : <Icon name={icon} />}
      {children}
    </span>
  );
}
