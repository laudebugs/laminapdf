// The window's document tabs (desktop shell S02): a strip over the top bar, one tab per open
// document, a close button on the active, hovered and focused tab, and a menu at the end that
// lists every tab and moves the shown one to another window. A tab's context menu (right-click,
// Shift+F10 or the menu key) shows that tab and opens the same menu. It is a tablist with a roving
// tabindex: one Tab stop, the arrow keys move between tabs, Home and End jump, Enter or Space
// shows the focused tab, Delete closes it. Tabs reorder by dragging within the strip, with a mark
// where the tab will go; a drag released anywhere else is the shell's (`onDragOut`): the desktop app
// moves the tab to the window it was dropped on, or to a new one. Dragging uses pointer events
// (`tab-drag.ts`). Tabs open, close and reorder with motion (`tab-motion.ts`).
//
// Only shells whose platform has `capabilities.multiWindow` render it (the desktop app); the web
// reader has one document per browser tab.

import { Icon } from "@laminapdf/icons/react";
import { type KeyboardEvent, type ReactNode, useRef, useState } from "react";
import { Button as AriaButton, MenuTrigger } from "react-aria-components";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry } from "../overlays/popover.js";
import { type TabDrag, type TabDragHandlers, useTabDrag } from "./tab-drag.js";
import { useTabMotion } from "./tab-motion.js";

/** One tab: the id the shell knows it by and the document's name. */
export interface DocumentTab {
  readonly id: string;
  readonly title: string;
  /** Its file couldn't be found: the tab shows a warning mark and the strip's menu says "Not found". */
  readonly missing?: boolean;
  /** A line under its entry in the strip's menu, such as the page it shows. */
  readonly detail?: string;
}

/** Props of {@link DocumentTabs}: the tabs, what showing, closing and moving them do, and what a
    drag does (`TabDragHandlers`). */
export interface DocumentTabsProps extends TabDragHandlers {
  /** The tabs, in order. */
  readonly tabs: readonly DocumentTab[];
  /** The shown tab's id; none while the window shows its start screen or an error. */
  readonly active: string | undefined;
  /** Show this tab. */
  readonly onActivate: (id: string) => void;
  /** Close this tab. */
  readonly onClose: (id: string) => void;
  /** Entries after the tab list in the strip's menu, such as "Move tab to new window". */
  readonly moves?: readonly MenuEntry<string>[];
  /** One of `moves` was chosen. */
  readonly onMove?: (id: string) => void;
  /** How shortcut chips are written. */
  readonly platform: KeyPlatform;
}

/** The prefix that tells a tab's entry in the strip's menu from the move entries. */
const TAB_ENTRY = "tab:";

/** The window's tab strip. See the module's comment for its keyboard and pointer behaviour. */
export function DocumentTabs({
  tabs,
  active,
  onActivate,
  onClose,
  onReorder,
  onDragStart,
  onDragOut,
  onDragCancel,
  moves = [],
  onMove,
  platform,
}: DocumentTabsProps): ReactNode {
  // The roving tab stop: the focused tab, else the active one, else the first.
  const [focused, setFocused] = useState<string>();
  const stop = tabs.find((t) => t.id === focused)?.id ?? active ?? tabs[0]?.id;
  const drag = useTabDrag(
    tabs.map((t) => t.id),
    { onReorder, onDragStart, onDragOut, onDragCancel },
  );
  const [menuOpen, setMenuOpen] = useState(false);
  // A tab's context menu: the tab shows, then the strip's menu opens with the moves for it.
  const openMenuFor = (id: string) => {
    onActivate(id);
    setMenuOpen(true);
  };
  // Focuses tab `id` in the strip that holds `from` (the tab a key was pressed on).
  const focusTab = (from: HTMLElement, id: string) => {
    setFocused(id);
    from
      .closest('[role="tablist"]')
      ?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(id)}"]`)
      ?.focus();
  };
  const onKeyDown = tabKeys(tabs, { focusTab, onActivate, onClose, openMenuFor });
  const list = useRef<HTMLDivElement>(null);
  useTabMotion(
    list,
    tabs.map((t) => t.id),
  );

  const menu: MenuEntry<string>[] = [...tabs.map(tabEntry), ...moves];
  return (
    <div className="paper-document-tabs">
      <div ref={list} role="tablist" aria-label={message("tabs.label")} className="paper-document-tabs-list">
        {tabs.map((tab, index) => (
          <OneTab
            key={tab.id}
            tab={tab}
            drop={dropMark(drag, index, tabs.length)}
            active={tab.id === active}
            stop={tab.id === stop}
            drag={drag}
            onActivate={onActivate}
            onClose={onClose}
            onFocus={setFocused}
            onKeyDown={onKeyDown}
            onContextMenu={openMenuFor}
          />
        ))}
      </div>
      <MenuTrigger isOpen={menuOpen} onOpenChange={setMenuOpen}>
        <AriaButton className="paper-document-tabs-menu" aria-label={message("tabs.menu", { count: tabs.length })}>
          <Icon name="chevron-down" />
        </AriaButton>
        <Menu
          aria-label={message("tabs.menu", { count: tabs.length })}
          items={menu}
          platform={platform}
          placement="bottom end"
          onAction={(id) => {
            if (id.startsWith(TAB_ENTRY)) onActivate(id.slice(TAB_ENTRY.length));
            else onMove?.(id);
          }}
        />
      </MenuTrigger>
    </div>
  );
}

/** A tab's entry in the strip's menu: a missing file flagged "Not found", else the tab's page. */
function tabEntry(tab: DocumentTab): MenuEntry<string> {
  const entry: MenuEntry<string> = { id: `${TAB_ENTRY}${tab.id}`, label: tab.title };
  if (tab.missing === true) return { ...entry, icon: "warning", description: message("tabs.notFound") };
  return tab.detail === undefined ? entry : { ...entry, description: tab.detail };
}

/** What the strip's keys do to its tabs. */
interface TabActions {
  focusTab(from: HTMLElement, id: string): void;
  onActivate(id: string): void;
  onClose(id: string): void;
  openMenuFor(id: string): void;
}

/** The tabs' key handler: arrows (mirrored right to left), Home, End, Enter, Space, the context-menu keys, Delete. */
function tabKeys(
  tabs: readonly DocumentTab[],
  act: TabActions,
): (event: KeyboardEvent<HTMLElement>, id: string) => void {
  return (event, id) => {
    const from = event.currentTarget;
    const index = tabs.findIndex((t) => t.id === id);
    const rtl = getComputedStyle(from).direction === "rtl";
    const step = (by: number) => tabs[(index + by + tabs.length) % tabs.length]?.id;
    const target = {
      ArrowRight: step(rtl ? -1 : 1),
      ArrowLeft: step(rtl ? 1 : -1),
      Home: tabs[0]?.id,
      End: tabs.at(-1)?.id,
    }[event.key];
    const contextMenu = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey);
    const show = () => act.onActivate(id);
    // Keyed by `KeyboardEvent.key` in lower case ("menu" for the context-menu keys).
    const others: Record<string, () => void> = {
      enter: show,
      " ": show,
      menu: () => act.openMenuFor(id),
      delete: () => {
        const next = step(1);
        act.onClose(id);
        if (next !== undefined && next !== id) act.focusTab(from, next);
      },
    };
    const action =
      target === undefined ? others[contextMenu ? "menu" : event.key.toLowerCase()] : () => act.focusTab(from, target);
    if (action === undefined) return;
    event.preventDefault();
    action();
  };
}

/** Where tab `index` (of `count`) shows the dragged tab would go: before it, or after the last
    tab; none where the drop would leave the order as it is. */
function dropMark(drag: TabDrag, index: number, count: number): "before" | "after" | undefined {
  const at = drag.insertAt;
  if (drag.dragged === undefined || at === undefined) return undefined;
  if (at === index) return "before";
  return at === count && index === count - 1 ? "after" : undefined;
}

/** One tab and its close button. */
function OneTab({
  tab,
  active,
  stop,
  drag,
  drop,
  onActivate,
  onClose,
  onFocus,
  onKeyDown,
  onContextMenu,
}: {
  readonly tab: DocumentTab;
  readonly active: boolean;
  /** Whether this tab is the strip's one Tab stop. */
  readonly stop: boolean;
  readonly drag: TabDrag;
  /** Whether the dragged tab would go before this tab or after it. */
  readonly drop: "before" | "after" | undefined;
  readonly onActivate: (id: string) => void;
  readonly onClose: (id: string) => void;
  readonly onFocus: (id: string) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>, id: string) => void;
  readonly onContextMenu: (id: string) => void;
}): ReactNode {
  return (
    <div
      role="presentation"
      className="paper-document-tab"
      data-tab-box={tab.id}
      data-active={active || undefined}
      data-dragging={drag.dragged === tab.id || undefined}
      data-drop={drop}
    >
      <div
        role="tab"
        id={`paper-tab-${tab.id}`}
        data-tab-id={tab.id}
        aria-selected={active}
        aria-label={tab.missing === true ? message("tabs.missingName", { name: tab.title }) : undefined}
        tabIndex={stop ? 0 : -1}
        title={tab.title}
        className="paper-document-tab-label"
        data-draggable={drag.enabled || undefined}
        onPointerDown={(event) => drag.pointerDown(event, tab.id)}
        onClick={() => {
          if (!drag.takeClick()) onActivate(tab.id);
        }}
        onFocus={() => onFocus(tab.id)}
        onKeyDown={(event) => onKeyDown(event, tab.id)}
        onContextMenu={(event) => {
          event.preventDefault();
          onContextMenu(tab.id);
        }}
      >
        {tab.missing === true ? <Icon name="warning" className="paper-document-tab-missing" /> : null}
        <span className="paper-document-tab-title">{tab.title}</span>
      </div>
      {/* Out of the tab (no control inside a tab) and out of the tab order: Delete on the focused
          tab and the strip's menu close tabs from the keyboard. */}
      <button
        type="button"
        tabIndex={-1}
        aria-hidden="true"
        title={message("tabs.close", { name: tab.title })}
        className="paper-document-tab-close"
        onClick={() => onClose(tab.id)}
      >
        <Icon name="close" />
      </button>
    </div>
  );
}
