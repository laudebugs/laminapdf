// The window's document tabs (desktop shell S02): a strip over the top bar, one tab per open
// document, a close button on the active, hovered and focused tab, and a menu at the end that
// lists every tab and moves the active one to another window. It is a tablist with a roving
// tabindex: one Tab stop, the arrow keys move between tabs, Home and End jump, Enter or Space
// shows the focused tab, Delete closes it. Tabs reorder by dragging within the strip.
//
// Shells with one window per document (the web reader) never render it; the desktop shell does,
// from two tabs on (`Platform.capabilities.multiWindow`).
import { type DragEvent, type KeyboardEvent, type ReactNode, useRef, useState } from "react";
import { Button as AriaButton, MenuTrigger } from "react-aria-components";
import { Icon } from "@laminapdf/icons/react";
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
  // The dragged tab: a ref for the drop handlers, which can run before React renders again, and
  // state for its dimmed look.
  const draggedRef = useRef<string | undefined>(undefined);
  const [dragged, setDraggedState] = useState<string>();
  const setDragged = (id: string | undefined) => {
    draggedRef.current = id;
    setDraggedState(id);
  };

  const focusTab = (id: string) => {
    setFocused(id);
    strip.current?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(id)}"]`)?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>, id: string) => {
    const index = tabs.findIndex((t) => t.id === id);
    const rtl = getComputedStyle(event.currentTarget).direction === "rtl";
    const step = (by: number) => tabs[(index + by + tabs.length) % tabs.length]?.id;
    const target = {
      ArrowRight: step(rtl ? -1 : 1),
      ArrowLeft: step(rtl ? 1 : -1),
      Home: tabs[0]?.id,
      End: tabs.at(-1)?.id,
    }[event.key];
    if (target !== undefined) {
      event.preventDefault();
      focusTab(target);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onActivate(id);
    } else if (event.key === "Delete") {
      event.preventDefault();
      const next = step(1);
      onClose(id);
      if (next !== undefined && next !== id) focusTab(next);
    }
  };

  const onDrop = (event: DragEvent, onto: string) => {
    event.preventDefault();
    const moving = draggedRef.current;
    setDragged(undefined);
    if (moving === undefined || moving === onto || onReorder === undefined) return;
    const ids = tabs.map((t) => t.id).filter((id) => id !== moving);
    ids.splice(ids.indexOf(onto), 0, moving);
    onReorder(ids);
  };

  const menu: MenuEntry<string>[] = [
    ...tabs.map((t) => ({ id: `${TAB_ENTRY}${t.id}`, label: t.title })),
    ...moves,
  ];
  return (
    <div className="paper-document-tabs">
      <div ref={strip} role="tablist" aria-label={message("tabs.label")} className="paper-document-tabs-list">
        {tabs.map((tab) => {
          const isActive = tab.id === active;
          return (
            <div
              key={tab.id}
              role="presentation"
              className="paper-document-tab"
              data-active={isActive || undefined}
              data-dragging={dragged === tab.id || undefined}
              onDragOver={(event) => {
                if (draggedRef.current !== undefined) event.preventDefault();
              }}
              onDrop={(event) => onDrop(event, tab.id)}
            >
              <div
                role="tab"
                id={`paper-tab-${tab.id}`}
                data-tab-id={tab.id}
                aria-selected={isActive}
                tabIndex={tab.id === stop ? 0 : -1}
                title={tab.title}
                className="paper-document-tab-label"
                draggable={onReorder !== undefined}
                onDragStart={(event) => {
                  setDragged(tab.id);
                  event.dataTransfer.effectAllowed = "move";
                }}
                onDragEnd={() => setDragged(undefined)}
                onClick={() => onActivate(tab.id)}
                onFocus={() => setFocused(tab.id)}
                onKeyDown={(event) => onKeyDown(event, tab.id)}
              >
                {tab.title}
              </div>
              {/* Out of the tab (no control inside a tab) and out of the tab order: Delete on
                  the focused tab and the strip's menu close tabs from the keyboard. */}
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
        })}
      </div>
      <MenuTrigger>
        <AriaButton
          className="paper-document-tabs-menu"
          aria-label={message("tabs.menu", { count: tabs.length })}
        >
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
