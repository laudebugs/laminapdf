// The reader's top bar (§7.1, Reader v0.1): the sidebar toggle and the file name on the left, with
// "Loading pages…" or "Locked" while those apply; the page input, zoom, Commands and More on the
// right. Without a document the file area shows what the shell gives it (the wordmark, or Back).
// It reads the reader controller and the viewer; its buttons run registered commands
// (`view.toggle-sidebar`, `palette.open`), whose handlers are the shell's. On a phone (§6.2) it keeps
// the file name and More, which opens the More sheet; the bottom bar holds the rest, pinch replaces
// the zoom, and the bar slides away while the document scrolls down and back when it scrolls up.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { Button as AriaButton } from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { IconButton } from "../components/button.js";
import { type KeyPlatform, ShortcutChip } from "../components/shortcut-chip.js";
import { ToggleButton } from "../components/toggle.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";
import { MoreSheet, usePhone } from "../shell/phone.js";
import { type ExternalStore, useStoreState } from "../state/hooks.js";
import type { ReaderState } from "../state/reader-machine.js";
import { LockedPageInput, PageInput } from "./page-input.js";
import type { ViewerControls } from "./viewer-controls.js";
import { ZoomControl } from "./zoom-control.js";

/** Props of `ReaderTopBar`. */
export interface ReaderTopBarProps<Id extends string> {
  /** The reader's state: a `ReaderController`, or any store of it. */
  reader: ExternalStore<ReaderState>;
  /** The viewer showing the open document, the commands' context's viewer; the page input and zoom show only with it. */
  viewer?: ViewerControls;
  /** How shortcut chips are written: the platform the app runs on. */
  keys: KeyPlatform;
  /** Whether visible pages are still being drawn: "Loading pages…" follows the file name. */
  isLoading?: boolean;
  /** The document's page labels, one per page, when it has any. */
  labels?: readonly string[];
  /** Whether the sidebar is open: the toggle shows it, and runs `view.toggle-sidebar` when pressed. */
  sidebarOpen?: boolean;
  /** The file area without a document: the wordmark, or Back. "No document open" when left out. */
  home?: ReactNode;
  /** A `DebugMenu`, which the shell shows only for `#debug`. */
  debug?: ReactNode;
  /** The More menu's items, gated by capability where they need one (`useGatedEntries`). */
  more: {
    /** The menu's entries. */
    readonly items: readonly MenuEntry<Id>[];
    /** Called with the chosen entry's id. */
    readonly onAction: (id: Id) => void;
  };
}

/** A viewer that never scrolls, for a top bar without one. */
const STILL: ExternalStore<{ readonly scrollTop?: number }> = {
  getState: () => ({}),
  subscribe: () => () => undefined,
};

/** Scrolled this far down past where it last turned, the phone's top bar hides. */
const HIDE_AFTER = 24;
/**
 * After the bar hides or shows, scrolling is ignored this long (ms): the viewer grows or shrinks by
 * the bar's height and re-fits its zoom, which moves the scroll position by half that, and would
 * otherwise toggle the bar back and forth.
 */
const SETTLE_MS = 400;

/** Whether the phone's top bar is out of the way: hidden while `viewer` scrolls down, back on the way up. */
function useHiddenWhileScrolling(viewer: ViewerControls | undefined, phone: boolean): boolean {
  const top = useStoreState<{ readonly scrollTop?: number }, number>(viewer ?? STILL, (s) => s.scrollTop ?? 0);
  const [hidden, setHidden] = useState(false);
  // Where the scrolling last turned: any move up shows the bar, a move down past HIDE_AFTER hides it.
  const turn = useRef(top);
  const quietUntil = useRef(0);
  const isHidden = useRef(false);
  useEffect(() => {
    const now = performance.now();
    if (!phone || now < quietUntil.current) {
      turn.current = top;
      return;
    }
    const moved = top - turn.current;
    if (moved < 0 || moved > HIDE_AFTER) {
      turn.current = top;
      if (isHidden.current !== moved > 0) quietUntil.current = now + SETTLE_MS;
      isHidden.current = moved > 0;
      setHidden(moved > 0);
    }
  }, [top, phone]);
  return phone && hidden && top > 0;
}

/** The phone's top bar: the file area, and More as a sheet. */
function PhoneTopBar<Id extends string>(props: ReaderTopBarProps<Id> & { hidden: boolean }): ReactNode {
  const state = useStoreState(props.reader, (s) => s);
  const [more, setMore] = useState(false);
  return (
    // Hidden, the bar is inert, so focus never rests in it out of sight.
    <header className="paper-top-bar" data-phone="" data-hidden={props.hidden || undefined} inert={props.hidden}>
      <div className="paper-top-bar-file">
        <FileArea state={state} isLoading={props.isLoading === true} home={props.home} />
      </div>
      {props.debug}
      <IconButton icon="more" label={message("reader.more")} onPress={() => setMore(true)} />
      <MoreSheet isOpen={more} onOpenChange={setMore} more={props.more} keys={props.keys} />
    </header>
  );
}

/** The reader's top bar. */
export function ReaderTopBar<Id extends string>(props: ReaderTopBarProps<Id>): ReactNode {
  const phone = usePhone();
  const hidden = useHiddenWhileScrolling(props.viewer, phone);
  return phone ? <PhoneTopBar {...props} hidden={hidden} /> : <DesktopTopBar {...props} />;
}

/** The top bar on a window wider than a phone. */
function DesktopTopBar<Id extends string>(props: ReaderTopBarProps<Id>): ReactNode {
  const state = useStoreState(props.reader, (s) => s);
  const { viewer, keys, more } = props;
  const sidebar = useCommand("view.toggle-sidebar");
  const palette = useCommand("palette.open");
  return (
    <header className="paper-top-bar">
      <div className="paper-top-bar-file">
        {state.phase !== "open" || sidebar.availability.state === "hidden" ? null : (
          <ToggleButton
            icon="sidebar"
            aria-label={message("reader.sidebar")}
            isSelected={props.sidebarOpen === true}
            isDisabled={sidebar.availability.state === "disabled"}
            onChange={() => sidebar.run()}
          />
        )}
        <FileArea state={state} isLoading={props.isLoading === true} home={props.home} />
      </div>
      <div className="paper-top-bar-tools">
        {state.phase === "needs-password" ? <LockedPageInput /> : null}
        {viewer === undefined || state.phase !== "open" ? null : (
          <>
            <PageInput viewer={viewer} {...(props.labels === undefined ? {} : { labels: props.labels })} />
            <ZoomControl viewer={viewer} keys={keys} />
          </>
        )}
        {palette.availability.state === "hidden" ? null : (
          <AriaButton className="paper-commands-button" onPress={() => palette.run()}>
            <span>{message("reader.commands")}</span>
            {palette.shortcut === undefined ? null : <ShortcutChip keys={palette.shortcut} platform={keys} />}
          </AriaButton>
        )}
        {props.debug}
        <MenuTrigger>
          <IconButton icon="more" label={message("reader.more")} />
          <Menu
            aria-label={message("reader.more")}
            items={more.items}
            onAction={more.onAction}
            platform={keys}
            placement="bottom end"
          />
        </MenuTrigger>
      </div>
    </header>
  );
}

/** The file name and what is happening to it, or the shell's home content without a document. */
function FileArea({ state, isLoading, home }: { state: ReaderState; isLoading: boolean; home: ReactNode }): ReactNode {
  if (state.phase === "empty")
    return home ?? <span className="paper-top-bar-empty">{message("reader.noDocument")}</span>;
  const loading = state.phase === "opening" || (state.phase === "open" && isLoading);
  const note =
    state.phase === "needs-password" ? message("reader.locked") : loading ? message("reader.loadingPages") : undefined;
  return (
    <>
      <span className="paper-top-bar-name" title={state.file.name}>
        {state.file.name}
      </span>
      {note === undefined ? null : <span className="paper-top-bar-note">{note}</span>}
    </>
  );
}
