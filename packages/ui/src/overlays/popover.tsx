// Popovers and menus (§7.26) on React Aria's popover: anchored to the button that opened them,
// portalled above whatever opened them (z-order `popover`), closed by Escape or a click outside,
// with focus back on the button.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import { COMPONENT } from "@laminapdf/tokens";
import { type ReactNode, useContext } from "react";
import {
  Dialog as AriaDialog,
  Menu as AriaMenu,
  MenuTrigger as AriaMenuTrigger,
  Popover as AriaPopover,
  DialogTrigger,
  Keyboard,
  MenuItem,
  OverlayTriggerStateContext,
  type Placement,
  Text,
} from "react-aria-components";
import { type KeyPlatform, keysText, type ShortcutKeys } from "../components/shortcut-chip.js";
import { LayerRegistration } from "./layers.js";

/**
 * Pairs a button with the `Popover` after it: `<PopoverTrigger><Button>View</Button><Popover …/></PopoverTrigger>`.
 * The button gets `aria-expanded` and opens the popover on press.
 */
export const PopoverTrigger = DialogTrigger;

/** Pairs a button with the `Menu` after it, as the menu button pattern: `aria-haspopup`, `aria-expanded`, arrows open it. */
export const MenuTrigger = AriaMenuTrigger;

/** Registers the open popover on the layer stack, closing it through its trigger's state. */
function PopoverLayer(): ReactNode {
  const state = useContext(OverlayTriggerStateContext);
  return state === null ? null : <LayerRegistration dismiss={() => state.close()} />;
}

/** Props of `Popover`. */
export interface PopoverProps {
  /** The popover's name: it is a dialog, so assistive technology announces it. */
  "aria-label": string;
  /** Its content: pickers, settings, a short form. */
  children: ReactNode;
  /** Where it sits against its button; below and at the start edge when left out. */
  placement?: Placement;
}

/** A popover dialog anchored to the button of its `PopoverTrigger`. */
export function Popover({ "aria-label": label, children, placement = "bottom start" }: PopoverProps): ReactNode {
  return (
    <AriaPopover className="paper-popover" placement={placement} offset={COMPONENT["tooltip-offset"]}>
      <PopoverLayer />
      <AriaDialog className="paper-popover-dialog" aria-label={label}>
        {children}
      </AriaDialog>
    </AriaPopover>
  );
}

/** One menu item. */
export interface MenuEntry<Id extends string> {
  /** Its id, passed to `onAction`. */
  id: Id;
  /** The visible label, which is also its name and its typeahead text. */
  label: string;
  /** A 16 px icon before the label; items without one keep the icon's space. */
  icon?: IconName;
  /** Its shortcut, shown as a chip at the end. */
  shortcut?: ShortcutKeys;
  /** Whether it can't be chosen now. Give `disabledReason` so people know why. */
  isDisabled?: boolean;
  /** Why it is disabled, shown under the label. */
  disabledReason?: string;
  /** A destructive action: shown in the danger colour. Put it last. */
  isDestructive?: boolean;
}

/** Props of `Menu`. */
export interface MenuProps<Id extends string> {
  /** The menu's name, usually its button's. */
  "aria-label": string;
  /** The items, in order. */
  items: readonly MenuEntry<Id>[];
  /** Called with the chosen item's id; the menu then closes. */
  onAction: (id: Id) => void;
  /** How shortcut chips are written: the platform the app runs on. */
  platform: KeyPlatform;
  /** Where it sits against its button; below and at the start edge when left out. */
  placement?: Placement;
}

/** A menu in a popover, opened by the button of its `MenuTrigger`. Arrows move, letters jump, Escape closes. */
export function Menu<Id extends string>({
  "aria-label": label,
  items,
  onAction,
  platform,
  placement = "bottom start",
}: MenuProps<Id>): ReactNode {
  return (
    <AriaPopover
      className="paper-popover paper-menu-popover"
      placement={placement}
      offset={COMPONENT["tooltip-offset"]}
    >
      <PopoverLayer />
      <AriaMenu
        className="paper-menu"
        aria-label={label}
        items={items}
        disabledKeys={items.filter((i) => i.isDisabled === true).map((i) => i.id)}
        onAction={(key) => onAction(key as Id)}
      >
        {(item) => (
          <MenuItem
            id={item.id}
            textValue={item.label}
            className="paper-menu-item"
            data-destructive={item.isDestructive === true || undefined}
          >
            {item.icon === undefined ? <span className="paper-menu-icon" /> : <Icon name={item.icon} />}
            <Text slot="label" className="paper-menu-label">
              {item.label}
            </Text>
            {/* React Aria's Keyboard slot links the chip to the item with aria-describedby. */}
            {item.shortcut === undefined ? null : (
              <Keyboard className="paper-shortcut-chip">{keysText(item.shortcut[platform], platform)}</Keyboard>
            )}
            {item.disabledReason === undefined ? null : (
              <Text slot="description" className="paper-menu-reason">
                {item.disabledReason}
              </Text>
            )}
          </MenuItem>
        )}
      </AriaMenu>
    </AriaPopover>
  );
}
