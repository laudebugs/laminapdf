// The empty state (§7.21): centred in the region it fills, an optional icon, a title, one sentence
// and at most a primary and a ghost action.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import { createElement, type ReactNode } from "react";

/** Props of `EmptyState`. */
export interface EmptyStateProps {
  /** The title: "This file has no outline". */
  title: string;
  /** The heading level the title takes in its region; 2 when left out. */
  headingLevel?: 2 | 3 | 4;
  /** One sentence: "Use Pages to move through it." */
  children?: ReactNode;
  /** A 32 px icon above the title. */
  icon?: IconName;
  /** At most one primary and one ghost `Button`. */
  actions?: ReactNode;
}

/** What a region shows when it has nothing to show. */
export function EmptyState({ title, headingLevel = 2, children, icon, actions }: EmptyStateProps): ReactNode {
  return (
    <div className="paper-empty-state">
      {icon === undefined ? null : <Icon name={icon} size={32} />}
      {createElement(`h${headingLevel}`, { className: "paper-empty-state-title" }, title)}
      {children === undefined ? null : <p className="paper-empty-state-text">{children}</p>}
      {actions === undefined ? null : <div className="paper-empty-state-actions">{actions}</div>}
    </div>
  );
}
