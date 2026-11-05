// Status bar items (§7.8): small muted buttons that each open a panel, and the page label. The
// network count, the repair details and the panels come from the app; these only show them.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button as AriaButton, type ButtonProps as AriaButtonProps } from "react-aria-components";

type ItemButtonProps = Omit<AriaButtonProps, "children" | "className" | "style">;

/** Props of `StatusBarItem`. */
export interface StatusBarItemProps extends ItemButtonProps {
  /** The visible text, which is also its name. */
  children: ReactNode;
  /** An icon before the text. */
  icon?: IconName;
}

/** A status-bar button: 12 px muted text, at least 24 px high (44 on touch). */
export function StatusBarItem({ children, icon, ...rest }: StatusBarItemProps): ReactNode {
  return (
    <AriaButton {...rest} className="paper-status-item">
      {icon === undefined ? null : <Icon name={icon} size={14} />}
      {children}
    </AriaButton>
  );
}

/** Props of `NetworkIndicator`. */
export interface NetworkIndicatorProps extends ItemButtonProps {
  /** The count, in words: "0 network requests for this document". */
  count: string;
  /** What the count means: "· Offline-safe: nothing uploaded". */
  explanation: string;
  /** A request went to an origin nobody expected: the shield and count turn the warning colour. */
  warning?: boolean;
}

/** The network indicator: a shield, the count in the text colour and an explanation. */
export function NetworkIndicator({ count, explanation, warning = false, ...rest }: NetworkIndicatorProps): ReactNode {
  return (
    <AriaButton {...rest} className="paper-status-item paper-network-indicator" data-warning={warning || undefined}>
      <Icon name="network-shield" size={15} />
      <span className="paper-network-count">{count}</span>
      <span>{explanation}</span>
    </AriaButton>
  );
}

/** Props of `RepairedNotice`. */
export interface RepairedNoticeProps extends ItemButtonProps {
  /** What happened: "Repaired on open". */
  label: string;
  /** The underlined word that says the button opens more: "Details". */
  details: string;
}

/** The repaired-file item: a wrench, what happened and an underlined "Details". */
export function RepairedNotice({ label, details, ...rest }: RepairedNoticeProps): ReactNode {
  return (
    <AriaButton {...rest} className="paper-status-item">
      <Icon name="repaired" size={14} />
      <span>
        {label} · <span className="paper-status-details">{details}</span>
      </span>
    </AriaButton>
  );
}

/** Props of `PageLabel`. */
export interface PageLabelProps {
  /** "Page 3 · Repairs backlog" or "Page 3 of 24". */
  children: ReactNode;
}

/** The page label: plain text with tabular numerals, so the bar doesn't shift as pages change. */
export function PageLabel({ children }: PageLabelProps): ReactNode {
  return <span className="paper-page-label">{children}</span>;
}
