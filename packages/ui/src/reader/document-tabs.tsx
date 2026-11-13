// The window's document tabs (desktop shell S02): a strip over the top bar, one tab per open
// document, a close button on the active, hovered and focused tab, and a menu at the end that
// lists every tab and moves the shown one to another window. A tab's context menu (right-click,
// Shift+F10 or the menu key) shows that tab and opens the same menu. It is a tablist with a roving
// tabindex: one Tab stop, the arrow keys move between tabs, Home and End jump, Enter or Space
// shows the focused tab, Delete closes it. Tabs reorder by dragging within the strip.
//
// Only shells whose platform has `capabilities.multiWindow` render it (the desktop app); the web
// reader has one document per browser tab.

import { Icon } from "@laminapdf/icons/react";
import { type DragEvent, type KeyboardEvent, type ReactNode, useRef, useState } from "react";
import { Button as AriaButton, MenuTrigger } from "react-aria-components";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry } from "../overlays/popover.js";

/** One tab: the id the shell knows it by and the document's name. */
export interface DocumentTab {
  readonly id: string;
  readonly title: string;
}

/** Props of {@link DocumentTabs}. */
export interface DocumentTabsProps {
  /** The tabs, in order. */
  readonly tabs: readonly DocumentTab[];
  /** The shown tab's id; none while the window shows its start screen or an error. */
  readonly active: string | undefined;
  /** Show this tab. */
  readonly onActivate: (id: string) => void;
  /** Close this tab. */
  readonly onClose: (id: string) => void;
  /** The tabs' new order, after one was dragged within the strip. */
  readonly onReorder?: (ids: readonly string[]) => void;
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
  moves = [],
  onMove,
  platform,
}: DocumentTabsProps): ReactNode {
  const strip = useRef<HTMLDivElement>(null);
  // The roving tab stop: the focused tab, else the active one, else the first.
  const [focused, setFocused] = useState<string>();
  const stop = tabs.find((t) => t.id === focused)?.id ?? active ?? tabs[0]?.id;
  const drag = useTabDrag(tabs, onReorder);
  const [menuOpen, setMenuOpen] = useState(false);
  // A tab's context menu: the tab shows, then the strip's menu opens with the moves for it.
  const openMenuFor = (id: string) => {
    onActivate(id);
    setMenuOpen(true);
  };
  const focusTab = (id: string) => {
    setFocused(id);
    strip.current?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(id)}"]`)?.focus();
  };
  const onKeyDown = tabKeys(tabs, { focusTab, onActivate, onClose, openMenuFor });

  const menu: MenuEntry<string>[] = [...tabs.map((t) => ({ id: `${TAB_ENTRY}${t.id}`, label: t.title })), ...moves];
  return (
    <div className="paper-document-tabs">
      <div ref={strip} role="tablist" aria-label={message("tabs.label")} className="paper-document-tabs-list">
        {tabs.map((tab) => (
          <OneTab
            key={tab.id}
            tab={tab}
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

/** What the strip's keys do to its tabs. */
interface TabActions {
  focusTab(id: string): void;
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
    const index = tabs.findIndex((t) => t.id === id);
    const rtl = getComputedStyle(event.currentTarget).direction === "rtl";
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
        if (next !== undefined && next !== id) act.focusTab(next);
      },
    };
    const action =
      target === undefined ? others[contextMenu ? "menu" : event.key.toLowerCase()] : () => act.focusTab(target);
    if (action === undefined) return;
    event.preventDefault();
    action();
  };
}

/** Reordering by drag within the strip: which tab is dragged, and the drop that reorders. */
interface TabDrag {
  readonly dragged: string | undefined;
  /** Whether `onReorder` was given: tabs are draggable only then. */
  readonly enabled: boolean;
  start(id: string): void;
  end(): void;
  /** A drag over a tab may drop there while a tab of this strip is dragged. */
  over(event: DragEvent): void;
  drop(event: DragEvent, onto: string): void;
}

function useTabDrag(tabs: readonly DocumentTab[], onReorder: DocumentTabsProps["onReorder"]): TabDrag {
  // A ref for the drop handlers, which can run before React renders again, and state for the
  // dragged tab's dimmed look.
  const draggedRef = useRef<string | undefined>(undefined);
  const [dragged, setDraggedState] = useState<string>();
  const setDragged = (id: string | undefined) => {
    draggedRef.current = id;
    setDraggedState(id);
  };
  return {
    dragged,
    enabled: onReorder !== undefined,
    start: setDragged,
    end: () => setDragged(undefined),
    over: (event) => {
      if (draggedRef.current !== undefined) event.preventDefault();
    },
    drop: (event, onto) => {
      event.preventDefault();
      const moving = draggedRef.current;
      setDragged(undefined);
      if (moving === undefined || moving === onto || onReorder === undefined) return;
      const all = tabs.map((t) => t.id);
      // Dropped on a tab after it: it goes after that tab; on one before it: before that tab.
      const after = all.indexOf(moving) < all.indexOf(onto);
      const ids = all.filter((id) => id !== moving);
      ids.splice(ids.indexOf(onto) + (after ? 1 : 0), 0, moving);
      onReorder(ids);
    },
  };
}

/** One tab and its close button. */
function OneTab({
  tab,
  active,
  stop,
  drag,
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
      data-active={active || undefined}
      data-dragging={drag.dragged === tab.id || undefined}
    >
      <div
        role="tab"
        id={`paper-tab-${tab.id}`}
        data-tab-id={tab.id}
        aria-selected={active}
        tabIndex={stop ? 0 : -1}
        title={tab.title}
        className="paper-document-tab-label"
        draggable={drag.enabled}
        onDragStart={(event) => {
          drag.start(tab.id);
          event.dataTransfer.effectAllowed = "move";
        }}
        onDragEnd={drag.end}
        onDragOver={drag.over}
        onDrop={(event) => drag.drop(event, tab.id)}
        onClick={() => onActivate(tab.id)}
        onFocus={() => onFocus(tab.id)}
        onKeyDown={(event) => onKeyDown(event, tab.id)}
        onContextMenu={(event) => {
          event.preventDefault();
          onContextMenu(tab.id);
        }}
      >
        {tab.title}
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
