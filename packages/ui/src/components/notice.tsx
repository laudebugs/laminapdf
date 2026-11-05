// Notices and the toast's look (§7.20). A notice is a strip inside a panel for a lasting state, and
// a status message (WCAG 4.1.3). A toast confirms something that already happened; the toast
// region announces it, and owns its timing and stacking, so the toast itself is not a live region.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";

/** A notice's status colour. */
export type NoticeTone = "neutral" | "positive" | "warning" | "danger";

const NOTICE_ICONS: Record<NoticeTone, IconName> = {
  neutral: "info",
  positive: "positive",
  warning: "warning",
  danger: "error",
};

/** Props of `Notice`. */
export interface NoticeProps {
  /** The lead words, in bold: "Approximate layout." */
  title: string;
  /** The sentence after the title. */
  children?: ReactNode;
  /** The tint and icon colour; `neutral` when left out. */
  tone?: NoticeTone;
}

/** An inline notice: a tinted strip with the tone's icon, a bold title and a sentence. */
export function Notice({ title, children, tone = "neutral" }: NoticeProps): ReactNode {
  return (
    <div role="status" className="paper-notice" data-tone={tone}>
      <Icon name={NOTICE_ICONS[tone]} />
      <p className="paper-notice-text">
        <strong className="paper-notice-title">{title}</strong> {children}
      </p>
    </div>
  );
}

/** A toast's one action, such as "Undo". */
export interface ToastAction {
  /** The visible label. */
  label: string;
  /** Called when the action is pressed. */
  onPress: () => void;
}

/** Props of `Toast`. */
export interface ToastProps {
  /** What happened: "Copied", "Highlights exported". */
  children: ReactNode;
  /** An optional action, shown as underlined text. */
  action?: ToastAction;
}

/**
 * A toast's look: dark and short. Put it inside the app's toast region, a polite live region that
 * exists before the toast arrives, so screen readers announce it. Never the only place an error is
 * reported.
 */
export function Toast({ children, action }: ToastProps): ReactNode {
  return (
    <div className="paper-toast">
      <span>{children}</span>
      {action === undefined ? null : (
        <AriaButton className="paper-toast-action" onPress={action.onPress}>
          {action.label}
        </AriaButton>
      )}
    </div>
  );
}
