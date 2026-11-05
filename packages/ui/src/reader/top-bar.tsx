// The reader's top bar (§7.1, Reader v0.1): the sidebar toggle and the file name on the left, with
// "Loading pages…" or "Locked" while those apply; the page input, zoom, Commands and More on the
// right. Without a document the file area shows what the shell gives it (the wordmark, or Back).
// It reads the reader controller and the viewer; what its buttons do is the shell's.

import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";
import { IconButton } from "../components/button.js";
import { type KeyPlatform, ShortcutChip, type ShortcutKeys } from "../components/shortcut-chip.js";
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
  /** The viewer showing the open document; the page input and zoom show only with it. */
  viewer?: ViewerControls;
  /** How shortcut chips are written: the platform the app runs on. */
  keys: KeyPlatform;
  /** Whether visible pages are still being drawn: "Loading pages…" follows the file name. */
  isLoading?: boolean;
  /** The document's page labels, one per page, when it has any. */
  labels?: readonly string[];
  /** The sidebar toggle, shown while a document is open. */
  sidebar?: { readonly isOpen: boolean; readonly onChange: (isOpen: boolean) => void };
  /** The file area without a document: the wordmark, or Back. "No document open" when left out. */
  home?: ReactNode;
  /** Opens the command palette; the Commands button shows only with it. */
  commands?: { readonly onPress: () => void; readonly shortcut: ShortcutKeys };
  /** The More menu's items, gated by capability where they need one (`useGatedEntries`). */
  more: { readonly items: readonly MenuEntry<Id>[]; readonly onAction: (id: Id) => void };
}

/** The reader's top bar. */
export function ReaderTopBar<Id extends string>(props: ReaderTopBarProps<Id>): ReactNode {
  const state = useStoreState(props.reader, (s) => s);
  const { viewer, keys, sidebar, commands, more } = props;
  return (
    <header className="paper-top-bar">
      <div className="paper-top-bar-file">
        {state.phase !== "open" || sidebar === undefined ? null : (
          <ToggleButton
            icon="sidebar"
            aria-label={message("reader.sidebar")}
            isSelected={sidebar.isOpen}
            onChange={sidebar.onChange}
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
        {commands === undefined ? null : (
          <AriaButton className="paper-commands-button" onPress={commands.onPress}>
            <span>{message("reader.commands")}</span>
            <ShortcutChip keys={commands.shortcut} platform={keys} />
          </AriaButton>
        )}
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
