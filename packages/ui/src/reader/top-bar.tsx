// The reader's top bar (§7.1, Reader v0.1): the sidebar toggle and the file name on the left, with
// "Loading pages…" or "Locked" while those apply; the page input, zoom, Commands and More on the
// right. Without a document the file area shows what the shell gives it (the wordmark, or Back).
// It reads the reader controller and the viewer; its buttons run registered commands
// (`view.toggle-sidebar`, `palette.open`), whose handlers are the shell's.

import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { IconButton } from "../components/button.js";
import { type KeyPlatform, ShortcutChip } from "../components/shortcut-chip.js";
import { ToggleButton } from "../components/toggle.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";
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

/** The reader's top bar. */
export function ReaderTopBar<Id extends string>(props: ReaderTopBarProps<Id>): ReactNode {
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
