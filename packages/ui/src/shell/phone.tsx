// The phone shell (design language §6.2, ADR 0024 §6, F03): below the phone breakpoint the status
// bar gives way to a bottom bar within the thumb's reach, with Search, Pages and More; Pages opens
// the rail as the Contents sheet and More opens a sheet of the top bar's More items. Each control
// appears only when the shell handles its command (Search waits for find). While text is selected
// by touch, the selection's own bar replaces it (UI/UX plan §5.3): Copy, Share, Search for and
// More, at the bottom, so the OS selection menu by the text is never covered.

import type { IconName } from "@laminapdf/icons";
import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useState } from "react";
import { Button as AriaButton, Menu as AriaMenu } from "react-aria-components";
import { PHONE_QUERY } from "../breakpoints.js";
import { useCommand } from "../commands/react.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { type MenuEntry, menuItem } from "../overlays/popover.js";
import { BottomSheet } from "../overlays/sheet.js";
import { useMediaQuery } from "../state/hooks.js";
import type { ShellLayout } from "./layout.js";

/** Whether the window is phone-sized now. */
export function usePhone(): boolean {
  return useMediaQuery(PHONE_QUERY);
}

/** The More items, and what runs one. */
export interface MoreItems<Id extends string> {
  /** The entries, gated by capability where they need one (`useGatedEntries`). */
  readonly items: readonly MenuEntry<Id>[];
  /** Runs the chosen entry. */
  readonly onAction: (id: Id) => void;
}

/** The More sheet: the top bar's More menu as a medium sheet of rows at the touch target height (R18). */
export function MoreSheet<Id extends string>({
  isOpen,
  onOpenChange,
  more,
  keys,
}: {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  more: MoreItems<Id>;
  keys: KeyPlatform;
}): ReactNode {
  return (
    <BottomSheet
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={message("reader.more")}
      closeLabel={message("phone.more.close")}
      resizeLabel={message("phone.sheet.resize")}
    >
      <AriaMenu
        className="paper-menu paper-sheet-menu"
        aria-label={message("reader.more")}
        items={more.items}
        disabledKeys={more.items.filter((i) => i.isDisabled === true).map((i) => i.id)}
        onAction={(key) => {
          onOpenChange(false);
          more.onAction(key as Id);
        }}
      >
        {(item) => menuItem(item, keys)}
      </AriaMenu>
    </BottomSheet>
  );
}

/**
 * One bottom-bar button: its icon over its label. `keepSelection` keeps the focus where it is on a
 * press, so pressing it doesn't take the text selection away.
 */
function BarButton(props: { icon: IconName; label: string; onPress: () => void; keepSelection?: boolean }): ReactNode {
  return (
    <AriaButton
      className="paper-bottom-bar-button"
      onPress={props.onPress}
      preventFocusOnPress={props.keepSelection === true}
    >
      <Icon name={props.icon} />
      <span>{props.label}</span>
    </AriaButton>
  );
}

/** Props of `PhoneBottomBar`. */
export interface PhoneBottomBarProps<Id extends string> {
  readonly layout: ShellLayout;
  /** Whether the rail has tabs: without them Pages stays out. */
  readonly rail: boolean;
  readonly more: MoreItems<Id>;
  readonly keys: KeyPlatform;
}

/** The phone's bottom bar: Search, Pages and More. */
export function PhoneBottomBar<Id extends string>({ layout, rail, more, keys }: PhoneBottomBarProps<Id>): ReactNode {
  const find = useCommand("find.open");
  const [moreOpen, setMoreOpen] = useState(false);
  return (
    <div className="paper-bottom-bar" role="toolbar" aria-label={message("phone.bar")}>
      {find.availability.state === "hidden" ? null : (
        <BarButton icon="find" label={message("phone.search")} onPress={() => find.run()} />
      )}
      {rail ? <BarButton icon="pages" label={message("phone.pages")} onPress={() => layout.setRailOpen(true)} /> : null}
      <BarButton icon="more" label={message("reader.more")} onPress={() => setMoreOpen(true)} />
      <MoreSheet isOpen={moreOpen} onOpenChange={setMoreOpen} more={more} keys={keys} />
    </div>
  );
}

/** Props of `SelectionActionBar`. */
export interface SelectionActionBarProps<Id extends string> {
  /** The More sheet's entries: the selection's other actions. */
  readonly more: MoreItems<Id>;
  readonly keys: KeyPlatform;
}

/**
 * The bottom bar while text is selected by touch: Copy, Share (where the device can), Search for
 * and More. Render it in the bottom bar's place for as long as the selection lasts.
 */
export function SelectionActionBar<Id extends string>({ more, keys }: SelectionActionBarProps<Id>): ReactNode {
  const copy = useCommand("edit.copy");
  const share = useCommand("selection.share");
  const search = useCommand("selection.search");
  const [moreOpen, setMoreOpen] = useState(false);
  const shown = (c: { availability: { state: string } }) => c.availability.state !== "hidden";
  return (
    <div className="paper-bottom-bar" role="toolbar" aria-label={message("selection.bar")}>
      {shown(copy) ? (
        <BarButton icon="copy" label={message("selection.copy")} onPress={() => copy.run()} keepSelection />
      ) : null}
      {shown(share) ? (
        <BarButton icon="share" label={message("selection.share")} onPress={() => share.run()} keepSelection />
      ) : null}
      {shown(search) ? (
        <BarButton icon="find" label={message("selection.search")} onPress={() => search.run()} keepSelection />
      ) : null}
      {more.items.length === 0 ? null : (
        <BarButton icon="more" label={message("reader.more")} onPress={() => setMoreOpen(true)} keepSelection />
      )}
      <MoreSheet isOpen={moreOpen} onOpenChange={setMoreOpen} more={more} keys={keys} />
    </div>
  );
}
