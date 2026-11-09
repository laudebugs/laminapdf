// The reader: @laminapdf/ui's chrome around @laminapdf/viewer's view. The home screen opens a file
// from Choose a file, a drop anywhere in the window or a paste; the canvas then shows the document,
// or the failure page when it can't be opened. A repaired file gets the status bar's notice and
// the repair details panel, docked as the shell's inspector. Focus mode (Ctrl+\ or the palette) hides
// the secondary toolbar. The rail holds the open document's thumbnails and outline (on a phone, the
// Contents sheet), and the page input takes its page labels. #debug adds the backend switch and the
// first page's timing.

import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  DebugMenu,
  FocusModeItem,
  Inspector,
  LeftRail,
  type ModeRegistry,
  message,
  OpenDropZone,
  OpenFailure,
  PasswordDialog,
  PhoneBottomBar,
  type Platform,
  type ReaderController,
  type ReaderError,
  ReaderShell,
  type ReaderState,
  ReaderTopBar,
  RegistryPalette,
  RemoteFailure,
  RepairDetails,
  RepairedNotice,
  registerReaderCommands,
  type ShellLayout,
  StatusBarItem,
  shellActions,
  useGatedEntries,
  useReaderState,
  useShell,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { APP_NAME, repairsText, windowTitle } from "./document.ts";
import { type FindActions, useFind } from "./find.tsx";
import { railTabs } from "./rail.tsx";
import { type Engine, type EngineState, remoteUrlOf } from "./session.ts";

/** How shortcut chips are written: Apple keys on Apple platforms, Ctrl elsewhere. */
const KEYS = /Mac|iPhone|iPad/.test(navigator.platform) ? "mac" : "other";

type MoreId = "open" | "rotate";

/** Mounts `viewer` into a box that fills the canvas, and shows it while `visible`. */
function ViewerHost({ viewer, visible }: { viewer: Viewer | undefined; visible: boolean }): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (viewer !== undefined && host.current !== null) viewer.mount(host.current);
  }, [viewer]);
  return <div ref={host} className="web-viewer" hidden={!visible} />;
}

/** Names the window after the open document (its title, else its file name); nothing leaves the page. */
function useWindowTitle(fileName: string | undefined, title: string | undefined): void {
  useEffect(() => {
    document.title = windowTitle(
      fileName === undefined ? undefined : { fileName, ...(title === undefined ? {} : { title }) },
    );
  }, [fileName, title]);
}

/** The registry with the reader's commands, and the context they run in: focus mode, the modes and the palette are the shell's. */
function useReaderCommands(
  platform: Platform,
  reader: ReaderController,
  viewer: Viewer | undefined,
  shell: { readonly layout: ShellLayout; readonly modes: ModeRegistry; readonly openPalette: () => void },
  rail: boolean,
  find: FindActions,
): { registry: CommandRegistry; context: CommandContext } {
  const [registry] = useState(() => {
    const commands = new CommandRegistry();
    registerReaderCommands(commands);
    return commands;
  });
  // A mode registered later gets its `mode.<id>` handler: the context follows the modes.
  const modes = useStoreState(shell.modes, (s) => s.modes);
  const context = useMemo<CommandContext>(
    () => ({
      platform,
      reader,
      ...(viewer === undefined ? {} : { viewer }),
      // Without rail tabs (no document open) the sidebar command has no handler and stays out of view.
      actions: {
        ...shellActions(shell.layout, shell.modes, { rail }),
        "palette.open": shell.openPalette,
        ...(viewer === undefined ? {} : find),
      },
    }),
    [platform, reader, viewer, shell, modes, rail, find],
  );
  return { registry, context };
}

/** The status bar's items: the repaired notice, and under #debug the first page's timing. */
function statusItems(engine: EngineState, debug: boolean, onRepairs: (() => void) | undefined): ReactNode[] {
  const items: ReactNode[] = [];
  if (onRepairs !== undefined) {
    items.push(
      <RepairedNotice
        key="repaired"
        label={message("repair.title")}
        details={message("repair.details")}
        onPress={onRepairs}
      />,
    );
  }
  const { timing } = engine;
  if (debug && timing !== undefined) {
    items.push(<StatusBarItem key="timing">{`${timing.backend} · page 1 in ${timing.firstPageMs} ms`}</StatusBarItem>);
  }
  return items;
}

/**
 * Whether a remote file failed because the browser refused to read it: its host sends no CORS
 * headers, which `fetch` can't tell apart from being offline or a failed DNS lookup.
 */
function refusedByHost(error: ReaderError): boolean {
  return error.code === "fetch-failed" && error.data?.reason === "network";
}

/** The page for a document that can't open: the remote explanation, or the failure page. */
function Failure({
  state,
  reader,
  platform,
}: {
  state: ReaderState & { readonly phase: "error" };
  reader: ReaderController;
  platform: Platform;
}): ReactNode {
  const { file, error } = state;
  const url = remoteUrlOf(file);
  if (url !== undefined && refusedByHost(error)) {
    return (
      <RemoteFailure
        reason="cors"
        url={url}
        onRetry={() => void reader.open(file)}
        onChooseFile={() => void reader.openFromPicker()}
      />
    );
  }
  return (
    <OpenFailure
      error={error}
      onTryAnother={() => reader.close()}
      onCopy={() => void platform.copyText(`${error.code}: ${error.message}`)}
    />
  );
}

/**
 * The repair details of `fileName`, docked as the inspector once the status bar's notice is pressed,
 * and how to show them; nothing for a file without repairs. The panel belongs to one file: it starts
 * closed for each, and closes when the file changes.
 */
function useRepairsPanel(
  layout: ShellLayout,
  platform: Platform,
  fileName: string | undefined,
  repairs: EngineState["repairs"],
): { inspector: ReactNode; showRepairs: (() => void) | undefined } {
  const shown = useStoreState(layout, (s) => s.inspector?.id === "repairs");
  // Another file's repairs never show: a new file closes the panel.
  useEffect(() => {
    if (layout.getState().inspector?.id === "repairs") layout.closeInspector();
  }, [fileName, layout]);
  if (fileName === undefined || repairs.length === 0) return { inspector: undefined, showRepairs: undefined };
  const showRepairs = () => layout.openInspector({ id: "repairs", selection: false });
  const inspector = shown ? (
    <Inspector layout={layout}>
      <RepairDetails
        fileName={fileName}
        repairs={repairs}
        onClose={() => layout.closeInspector()}
        onCopy={() => void platform.copyText(repairsText(repairs))}
      />
    </Inspector>
  ) : undefined;
  return { inspector, showRepairs };
}

/** The reader over `engine`, on `platform`; `debug` shows the debug menu (`#debug`). */
export function App({ engine, platform, debug }: { engine: Engine; platform: Platform; debug: boolean }): ReactNode {
  const { reader } = engine;
  const state = useReaderState(reader);
  const engineState = useStoreState(engine, (s) => s);
  const { backend, viewer, repairs } = engineState;
  const [palette, setPalette] = useState(false);
  const { layout, modes } = useShell(platform.storage);
  const [shell] = useState(() => ({ layout, modes, openPalette: () => setPalette(true) }));
  const sidebar = useStoreState(layout, (s) => s.rail.open);
  const more = useGatedEntries<MoreId>([
    { id: "open", label: message("reader.open.choose") },
    { id: "rotate", label: message("reader.rotate"), icon: "rotate-right" },
  ]);
  const moreItems = { items: more, onAction: (id: MoreId) => (id === "open" ? reader.close() : viewer?.rotate(90)) };
  const open = state.phase === "open";
  const shown = open ? viewer : undefined;
  // The rail's panels are made again only when the view or the strip changes.
  const { strip } = engineState;
  const find = useFind(shown, open ? state.file : undefined);
  const tabs = useMemo(
    () => railTabs(engine, layout, open ? { strip, viewer } : undefined, find.open),
    [engine, layout, open, strip, viewer, find.open],
  );
  const labels = useMemo(() => {
    const pages = engineState.pages;
    return pages.some((p) => p.label !== undefined) ? pages.map((p) => p.label ?? String(p.index + 1)) : undefined;
  }, [engineState.pages]);
  const fileName = open ? state.file.name : undefined;
  const commands = useReaderCommands(platform, reader, shown, shell, tabs.length > 0, find.actions);
  useWindowTitle(fileName, engineState.title);
  const { inspector, showRepairs } = useRepairsPanel(layout, platform, fileName, repairs);
  const status = statusItems(engineState, debug, showRepairs);
  const debugMenu = debug ? (
    <DebugMenu backend={backend} onBackendChange={(next) => void engine.switchBackend(next)} keys={KEYS} />
  ) : undefined;
  return (
    <CommandsProvider registry={commands.registry} context={commands.context} keys={KEYS}>
      <ReaderShell
        layout={layout}
        modes={modes}
        {...(find.bar === undefined ? {} : { secondaryToolbar: find.bar })}
        topBar={
          <ReaderTopBar
            reader={reader}
            {...(shown === undefined ? {} : { viewer: shown })}
            keys={KEYS}
            sidebarOpen={sidebar}
            findOpen={find.open}
            {...(labels === undefined ? {} : { labels })}
            home={<span className="web-wordmark">{APP_NAME}</span>}
            debug={debugMenu}
            more={moreItems}
          />
        }
        rail={<LeftRail layout={layout} tabs={tabs} />}
        bottomBar={<PhoneBottomBar layout={layout} rail={tabs.length > 0} more={moreItems} keys={KEYS} />}
        {...(inspector === undefined ? {} : { inspector })}
        status={
          <>
            {status}
            <FocusModeItem layout={layout} />
          </>
        }
      >
        {state.phase === "empty" && engineState.linkProblem === undefined ? (
          <OpenDropZone reader={reader} keys={KEYS}>
            <p className="web-privacy">{message("reader.open.privacy")}</p>
          </OpenDropZone>
        ) : null}
        {state.phase === "empty" && engineState.linkProblem !== undefined ? (
          <RemoteFailure
            reason={engineState.linkProblem}
            onHome={() => engine.dismissLinkProblem()}
            onChooseFile={() => void reader.openFromPicker()}
          />
        ) : null}
        {state.phase === "error" ? <Failure state={state} reader={reader} platform={platform} /> : null}
        <ViewerHost viewer={viewer} visible={open} />
        <PasswordDialog reader={reader} />
      </ReaderShell>
      <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
    </CommandsProvider>
  );
}
