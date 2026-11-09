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
  ExternalLinkDialog,
  type ExternalStore,
  FocusModeItem,
  type GatedEntry,
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
  ToastQueue,
  ToastRegion,
  useGatedEntries,
  useReaderState,
  useShell,
  useStoreState,
} from "@laminapdf/ui";
import type { HistoryState, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { useAnnouncements } from "./announce.tsx";
import { APP_NAME, repairsText, windowTitle } from "./document.ts";
import { type FindActions, useFind } from "./find.tsx";
import type { NetworkMonitor } from "./network/monitor.ts";
import { NETWORK_PANEL, useNetworkPanel } from "./network/panel.tsx";
import { useOffline } from "./offline/offline.tsx";
import type { AppUpdates } from "./offline/updates.ts";
import { type PageInfo, pageLabel } from "./outline.ts";
import { type Output, type OutputCommand, useOutput } from "./output.tsx";
import { railTabs } from "./rail.tsx";
import type { SelectionSurfaces } from "./selection.tsx";
import { useSelection } from "./selection.tsx";
import { type Engine, type EngineState, remoteUrlOf } from "./session.ts";
import { type Settings, useSettings } from "./settings.tsx";

/** How shortcut chips are written: Apple keys on Apple platforms, Ctrl elsewhere. */
const KEYS = /Mac|iPhone|iPad/.test(navigator.platform) ? "mac" : "other";

type MoreId = "save-copy" | "print" | "properties" | "open" | "rotate" | "settings" | "shortcuts";

/** The More menu's commands, by item. */
const MORE_COMMANDS = {
  "save-copy": "file.save-copy",
  print: "file.print",
  properties: "file.properties",
} as const satisfies Record<Exclude<MoreId, "open" | "rotate" | "settings" | "shortcuts">, OutputCommand>;

/**
 * The More menu (R09): Download a copy, Print… and Document properties, which need an open
 * document, then the reader's own items.
 */
function moreEntries(open: boolean): GatedEntry<MoreId>[] {
  const needsDocument = open ? {} : { isDisabled: true, disabledReason: message("command.reason.noDocument") };
  return [
    { id: "save-copy", label: message("command.file.save-copy"), icon: "download", ...needsDocument },
    {
      id: "print",
      label: message("print.print"),
      icon: "printer",
      shortcut: { mac: "Mod+P", other: "Mod+P" },
      ...needsDocument,
    },
    { id: "properties", label: message("menu.properties"), icon: "info", ...needsDocument },
    { id: "open", label: message("reader.open.choose") },
    { id: "rotate", label: message("reader.rotate"), icon: "rotate-right" },
    { id: "settings", label: message("command.app.settings"), icon: "settings" },
    { id: "shortcuts", label: message("command.help.shortcuts"), icon: "keyboard" },
  ];
}

/** Mounts `viewer` into a box that fills the canvas, and shows it while `visible`. */
function ViewerHost({ viewer, visible }: { viewer: Viewer | undefined; visible: boolean }): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (viewer !== undefined && host.current !== null) viewer.mount(host.current);
  }, [viewer]);
  return <div ref={host} className="web-viewer" hidden={!visible} />;
}

/** A history with nowhere to go, for a top bar without a view. */
const NOWHERE: HistoryState = { current: undefined, back: undefined, forward: undefined };
const NO_HISTORY: ExternalStore<HistoryState> = { getState: () => NOWHERE, subscribe: () => () => undefined };

/**
 * Back and Forward (R07): where each goes, as page labels, and their handlers. Both step the
 * browser's history, which holds the view's jumps (open/history.ts), so the reader's buttons and
 * the browser's own do the same thing; with nowhere to go in this document they do nothing, so
 * Back never leaves the reader.
 */
function useHistory(
  viewer: Viewer | undefined,
  pages: readonly PageInfo[],
): {
  history: { readonly back: string | undefined; readonly forward: string | undefined };
  actions: Readonly<Record<"nav.back" | "nav.forward", () => void>>;
} {
  const state = useStoreState(viewer?.history ?? NO_HISTORY, (s) => s);
  const label = (entry: HistoryState["back"]) => (entry === undefined ? undefined : pageLabel(pages, entry.point.page));
  const actions = useMemo(
    () => ({
      "nav.back": () => {
        if (viewer?.history.getState().back !== undefined) window.history.back();
      },
      "nav.forward": () => {
        if (viewer?.history.getState().forward !== undefined) window.history.forward();
      },
    }),
    [viewer],
  );
  return { history: { back: label(state.back), forward: label(state.forward) }, actions };
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
  /** The handlers that need an open document: find, Back and Forward, print, download, properties and the selection's. */
  documentActions: FindActions &
    ReturnType<typeof useHistory>["actions"] &
    Output["actions"] &
    SelectionSurfaces["actions"],
  /** The handlers that work with or without a document: the settings'. */
  appActions: Settings["actions"],
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
        ...appActions,
        ...(viewer === undefined ? {} : documentActions),
        "privacy.network-activity": () => shell.layout.openInspector({ id: NETWORK_PANEL, selection: false }),
      },
    }),
    [platform, reader, viewer, shell, modes, rail, documentActions, appActions],
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

/** The reader's dialogs: the password prompt, and the confirmation before a web link opens. */
function ReaderDialogs({
  engine,
  link,
  platform,
}: {
  engine: Engine;
  link: EngineState["externalLink"];
  platform: Platform;
}): ReactNode {
  return (
    <>
      <PasswordDialog reader={engine.reader} />
      <ExternalLinkDialog
        link={link}
        onOpen={(href) => void platform.openExternal(href).catch(reportError)}
        onCopy={(href) => void platform.copyText(href).catch(reportError)}
        onClose={() => engine.closeExternalLink()}
      />
    </>
  );
}

/** The inspector's panel, the status bar's items (the network indicator first) and the offline notice (R15). */
function usePanels(
  layout: ShellLayout,
  platform: Platform,
  network: NetworkMonitor,
  fileName: string | undefined,
  engine: EngineState,
  debug: boolean,
  offline: { readonly updates: AppUpdates; readonly toasts: ToastQueue; readonly documentOpen: boolean },
): { inspector: ReactNode; status: ReactNode[]; notice: ReactNode } {
  const { inspector: repairsPanel, showRepairs } = useRepairsPanel(layout, platform, fileName, engine.repairs);
  const { indicator, inspector: networkPanel } = useNetworkPanel(layout, platform, network);
  const connection = useOffline(offline.updates, offline.toasts, offline.documentOpen);
  // One inspector at a time: whichever panel the layout holds draws, the other draws nothing.
  const inspector = repairsPanel ?? networkPanel;
  return {
    inspector,
    status: [
      ...(indicator === undefined ? [] : [indicator]),
      ...statusItems(engine, debug, showRepairs),
      ...connection.status,
    ],
    notice: connection.notice,
  };
}

/**
 * The More menu and what its document items need: the page labels, the output's handlers and
 * dialogs (print, download, properties), and the toasts they end with.
 */
function useDocumentMenu(
  engine: Engine,
  platform: Platform,
  state: ReaderState,
  shown: Viewer | undefined,
  appActions: Settings["actions"],
): {
  toasts: ToastQueue;
  labels: readonly string[] | undefined;
  output: Output;
  moreItems: { items: ReturnType<typeof useGatedEntries<MoreId>>; onAction: (id: MoreId) => void };
} {
  const pages = useStoreState(engine, (s) => s.pages);
  const [toasts] = useState(() => new ToastQueue());
  const labels = useMemo(
    () => (pages.some((p) => p.label !== undefined) ? pages.map((_, i) => pageLabel(pages, i)) : undefined),
    [pages],
  );
  const document =
    state.phase === "open" && shown !== undefined
      ? { fileName: state.file.name, pages, labels, viewer: shown }
      : undefined;
  const output = useOutput(engine, platform, toasts, document);
  const items = useGatedEntries<MoreId>(moreEntries(document !== undefined));
  const onAction = (id: MoreId) => {
    if (id === "open") engine.reader.close();
    else if (id === "rotate") engine.getState().viewer?.rotate(90);
    else if (id === "settings") appActions["app.settings"]?.(undefined);
    else if (id === "shortcuts") appActions["help.shortcuts"]?.(undefined);
    else output.actions[MORE_COMMANDS[id]]();
  };
  return { toasts, labels, output, moreItems: { items, onAction } };
}

/** The debug menu (#debug): which backend draws the pages. */
function EngineDebugMenu({ engine, backend }: { engine: Engine; backend: EngineState["backend"] }): ReactNode {
  return <DebugMenu backend={backend} onBackendChange={(next) => void engine.switchBackend(next)} keys={KEYS} />;
}

/** What the canvas shows: the home, a link's failure, an open failure, and the view. */
function Canvas({
  engine,
  state,
  platform,
  viewer,
}: {
  engine: Engine;
  state: ReaderState;
  platform: Platform;
  viewer: Viewer | undefined;
}): ReactNode {
  const { reader } = engine;
  const linkProblem = useStoreState(engine, (s) => s.linkProblem);
  return (
    <>
      {state.phase === "empty" && linkProblem === undefined ? (
        <OpenDropZone reader={reader} keys={KEYS}>
          <p className="web-privacy">{message("reader.open.privacy")}</p>
        </OpenDropZone>
      ) : null}
      {state.phase === "empty" && linkProblem !== undefined ? (
        <RemoteFailure
          reason={linkProblem}
          onHome={() => engine.dismissLinkProblem()}
          onChooseFile={() => void reader.openFromPicker()}
        />
      ) : null}
      {state.phase === "error" ? <Failure state={state} reader={reader} platform={platform} /> : null}
      <ViewerHost viewer={viewer} visible={state.phase === "open"} />
    </>
  );
}

/** Props of `App`. */
export interface AppProps {
  readonly engine: Engine;
  readonly platform: Platform;
  readonly network: NetworkMonitor;
  /** The service worker's update flow (R15). */
  readonly updates: AppUpdates;
  /** Shows the debug menu (`#debug`). */
  readonly debug: boolean;
}

/** The rail's tabs for the open document, made again only when the view, the strip or find changes. */
function useRailTabs(engine: Engine, layout: ShellLayout, open: boolean, finding: boolean) {
  const { strip, viewer } = useStoreState(engine, (s) => s);
  return useMemo(
    () => railTabs(engine, layout, open ? { strip, viewer, finding } : undefined),
    [engine, layout, open, strip, viewer, finding],
  );
}

/** The reader over `engine`, on `platform`. */
export function App({ engine, platform, network, updates, debug }: AppProps): ReactNode {
  const { reader } = engine;
  const state = useReaderState(reader);
  const engineState = useStoreState(engine, (s) => s);
  const { backend, viewer } = engineState;
  const [palette, setPalette] = useState(false);
  const { layout, modes } = useShell(platform.storage);
  const [shell] = useState(() => ({ layout, modes, openPalette: () => setPalette(true) }));
  const sidebar = useStoreState(layout, (s) => s.rail.open);
  const open = state.phase === "open";
  const shown = open ? viewer : undefined;
  const settings = useSettings(shell.openPalette);
  const { toasts, labels, output, moreItems } = useDocumentMenu(engine, platform, state, shown, settings.actions);
  const find = useFind(shown, open ? state.file : undefined);
  const tabs = useRailTabs(engine, layout, open, find.open);
  const fileName = open ? state.file.name : undefined;
  const rail = tabs.length > 0;
  const history = useHistory(shown, engineState.pages);
  const selection = useSelection(shown, find, platform, KEYS);
  const commands = useReaderCommands(
    platform,
    reader,
    shown,
    shell,
    rail,
    { ...find.actions, ...history.actions, ...output.actions, ...selection.actions },
    settings.actions,
  );
  useWindowTitle(fileName, engineState.title);
  const liveRegions = useAnnouncements(shown, open ? state.file : undefined, engineState.pages);
  const { inspector, status, notice } = usePanels(layout, platform, network, fileName, engineState, debug, {
    updates,
    toasts,
    documentOpen: open,
  });
  return (
    <CommandsProvider
      registry={commands.registry}
      context={commands.context}
      keys={KEYS}
      singleKeys={settings.appearance.singleKeys}
    >
      <ReaderShell
        layout={layout}
        modes={modes}
        {...(find.bar === undefined ? {} : { secondaryToolbar: find.bar })}
        notice={notice}
        topBar={
          <ReaderTopBar
            reader={reader}
            {...(shown === undefined ? {} : { viewer: shown })}
            keys={KEYS}
            sidebarOpen={sidebar}
            findOpen={find.open}
            {...(shown === undefined ? {} : { history: history.history })}
            {...(labels === undefined ? {} : { labels })}
            home={<span className="web-wordmark">{APP_NAME}</span>}
            debug={debug ? <EngineDebugMenu engine={engine} backend={backend} /> : undefined}
            view={settings.viewButton}
            more={moreItems}
          />
        }
        rail={<LeftRail layout={layout} tabs={tabs} />}
        bottomBar={selection.bar ?? <PhoneBottomBar layout={layout} rail={rail} more={moreItems} keys={KEYS} />}
        {...(inspector === undefined ? {} : { inspector })}
        status={
          <>
            {status}
            <FocusModeItem layout={layout} />
          </>
        }
      >
        <Canvas engine={engine} state={state} platform={platform} viewer={viewer} />
        <ReaderDialogs engine={engine} link={engineState.externalLink} platform={platform} />
        {output.dialogs}
      </ReaderShell>
      <ToastRegion queue={toasts} aria-label={message("shell.toasts")} />
      <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
      {settings.dialogs}
      {liveRegions}
      {selection.toolbar}
    </CommandsProvider>
  );
}
