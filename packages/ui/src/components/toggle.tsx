// Buttons that stay pressed: a plain toggle, and the secondary toolbar's tool toggle (§7.24).

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import {
  ToggleButton as AriaToggleButton,
  type ToggleButtonProps as AriaToggleButtonProps,
} from "react-aria-components";

type BaseProps = Omit<AriaToggleButtonProps, "children" | "className" | "style">;

/** Props of `ToggleButton`. */
export interface ToggleButtonProps extends BaseProps {
  /** The visible label; leave it out for an icon alone and name the button with `aria-label`. */
  children?: ReactNode;
  /** An icon before the label. */
  icon?: IconName;
}

/** A button that stays pressed (`aria-pressed`) until pressed again: Sidebar, Reader, bold. */
export function ToggleButton({ children, icon, ...rest }: ToggleButtonProps): ReactNode {
  return (
    <AriaToggleButton {...rest} className="paper-toggle">
      {icon === undefined ? null : <Icon name={icon} size={16} />}
      {children === undefined ? null : <span className="paper-toggle-label">{children}</span>}
    </AriaToggleButton>
  );
}

/** Props of `ToolToggle`. */
export interface ToolToggleProps extends BaseProps {
  /** The tool's name, shown beside its icon. */
  children: ReactNode;
  /** The tool's icon. */
  icon: IconName;
}

/** A tool in a mode's secondary toolbar: a 16 px icon and a 13 px label, pressed while the tool is on. */
export function ToolToggle({ children, icon, ...rest }: ToolToggleProps): ReactNode {
  return (
    <AriaToggleButton {...rest} className="paper-tool-toggle">
      <Icon name={icon} size={16} />
      <span className="paper-toggle-label">{children}</span>
    </AriaToggleButton>
  );
}
