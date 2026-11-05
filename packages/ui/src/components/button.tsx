// Text buttons and icon buttons (design language §7.3, §7.4), over React Aria's Button: press,
// keyboard and pending state come from it, the look from button.css.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button as AriaButton, type ButtonProps as AriaButtonProps } from "react-aria-components";

/** A text button's role: one `primary` per view; `danger-filled` only as a destructive dialog's confirm. */
export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "danger-filled";

/** A text button's height in px: 32 by default, 30 in the inspector, 26 in the top bar. */
export type ButtonSize = 32 | 30 | 26;

/** Props of `Button`. */
export interface ButtonProps extends Omit<AriaButtonProps, "children" | "className" | "style"> {
  /** The visible label. */
  children: ReactNode;
  /** The button's role; `secondary` when left out. */
  variant?: ButtonVariant;
  /** The height on desktop; touch density makes every button at least 44 px. */
  size?: ButtonSize;
  /** An icon before the label. */
  icon?: IconName;
}

/**
 * A text button. While `isPending`, it keeps its width and focus, can't be pressed, and is
 * announced as busy.
 */
export function Button({ children, variant = "secondary", size = 32, icon, ...rest }: ButtonProps): ReactNode {
  return (
    <AriaButton {...rest} className="paper-button" data-variant={variant} data-size={size}>
      {icon === undefined ? null : <Icon name={icon} size={16} />}
      <span className="paper-button-label">{children}</span>
    </AriaButton>
  );
}

/** An icon button's size in px: 28 in the top bar, 26 for zoom, 32 on larger surfaces. */
export type IconButtonSize = 26 | 28 | 32;

/** Props of `IconButton`. */
export interface IconButtonProps extends Omit<AriaButtonProps, "children" | "className" | "style" | "aria-label"> {
  /** The accessible name: a verb phrase such as "Find in document". Required, since nothing else names it. */
  label: string;
  /** The icon. */
  icon: IconName;
  /** The size on desktop; touch density makes it 44 px with a 22 px icon. */
  size?: IconButtonSize;
}

/** A button that shows only an icon; `label` names it for assistive technology. */
export function IconButton({ label, icon, size = 28, ...rest }: IconButtonProps): ReactNode {
  return (
    <AriaButton {...rest} aria-label={label} className="paper-icon-button" data-size={size}>
      {/* button.css sizes the icon: 16 px at 26, 18 px otherwise, 22 px on touch. */}
      <Icon name={icon} />
    </AriaButton>
  );
}
