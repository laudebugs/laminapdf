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
  type MenuEntry,
  type ModeRegistry,
  message,
  OpenDropZone,
  OpenFailure,
  PasswordDialog,
  PhoneBottomBar,
  type Platform,
  type PlatformFile,
  type RailTab,
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
import { type CapabilityRoute, useCapabilityRoute } from "./capabilities.tsx";
import { APP_NAME, repairsText, windowTitle } from "./document.ts";
import { type EditActions, type Editing, type EditMenuId, useEditing } from "./editing.tsx";
import { type FindActions, useFind } from "./find.tsx";
import type { NetworkMonitor } from "./network/monitor.ts";
import { NETWORK_PANEL, useNetworkPanel } from "./network/panel.tsx";
import { clearDocumentCache } from "./offline/document-cache.ts";
import { useInstall } from "./offline/install.tsx";
import { useOffline } from "./offline/offline.tsx";
import type { AppUpdates } from "./offline/updates.ts";
import { type PageInfo, pageLabel } from "./outline.ts";
import { type Output, type OutputCommand, useOutput } from "./output.tsx";
import { usePageAppearance } from "./page-appearance.tsx";
import { railTabs } from "./rail.tsx";
import { type ReaderMode, useReaderMode } from "./reader-mode.tsx";
import { type Recent, useRecent } from "./recent.tsx";
import type { SelectionSurfaces } from "./selection.tsx";
import { useSelection } from "./selection.tsx";
import { type Engine, type EngineState, remoteUrlOf } from "./session.ts";
import { type Settings, useSettings } from "./settings.tsx";

/** How shortcut chips are written: Apple keys on Apple platforms, Ctrl elsewhere. */
const KEYS = /Mac|iPhone|iPad/.test(navigator.platform) ? "mac" : "other";

/** The website's privacy page, which states the reader's privacy promises. */
const PRIVACY_URL = "https://laminapdf.com/privacy/";

/** The More menu's own items; the edit items join them (`withEditItems`). */
type ReaderMoreId =
  | "save-copy"
  | "print"
  | "properties"
  | "install"
  | "open"
  | "rotate"
  | "settings"
  | "shortcuts"
  | "capabilities"
  | "privacy";

/** The More menu's items that aren't one of the output's commands: the menu handles each itself. */
type OwnMoreId = "install" | "open" | "rotate" | "settings" | "shortcuts" | "capabilities" | "privacy";

/** The More menu's commands, by item. */
const MORE_COMMANDS = {
  "save-copy": "file.save-copy",
  print: "file.print",
  properties: "file.properties",
} as const satisfies Record<Exclude<ReaderMoreId, OwnMoreId>, OutputCommand>;

/** Every item of the More menu: the edit items and its own. */
type MoreId = EditMenuId | ReaderMoreId;

/**
 * The More menu (R09): Download a copy, Print… and Document properties, which need an open
 * document; then the reader's own items. The edit items (the web's Edit menu, O01) go first once
 * a document is open (`withEditItems`).
 */
function moreEntries(open: boolean, installable: boolean): GatedEntry<ReaderMoreId>[] {
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
    // R15: only while the browser offers an install.
    ...(installable
      ? [{ id: "install" as const, label: message("command.app.install"), icon: "download" as const }]
      : []),
    { id: "open", label: message("reader.open.choose") },
    { id: "rotate", label: message("reader.rotate"), icon: "rotate-right" },
    { id: "settings", label: message("command.app.settings"), icon: "settings" },
    { id: "shortcuts", label: message("command.help.shortcuts"), icon: "keyboard" },
    { id: "capabilities", label: message("command.help.capabilities"), icon: "info" },
    { id: "privacy", label: message("reader.privacy"), icon: "network-shield" },
  ];
}

/**
 * Mounts `viewer` into a box that fills the canvas, and shows it while `visible`. While
 * `covered` (reader mode's column is over it) it keeps its size and place, unseen and inert, so
 * leaving reader mode can move it at once.
 */
function ViewerHost({
  viewer,
  visible,
  covered,
}: {
  viewer: Viewer | undefined;
  visible: boolean;
  covered: boolean;
}): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (viewer !== undefined && host.current !== null) viewer.mount(host.current);
  }, [viewer]);
  return (
    <div ref={host} className="web-viewer" hidden={!visible} inert={covered} data-covered={covered || undefined} />
  );
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
  actions: Readonly<Record<"nav.back" | "nav.forward" | "nav.follow-link", () => void>>;
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
      // The previewed link, else the focused one (R07).
      "nav.follow-link": () => {
        viewer?.followLinkPreview();
      },
    }),
    [viewer],
  );
  return { history: { back: label(state.back), forward: label(state.forward) }, actions };
}

/**
 * Names the window after the open document (its title, else its file name), with a "•" while it
 * has unsaved changes; nothing leaves the page.
 */
function useWindowTitle(fileName: string | undefined, title: string | undefined, unsaved: boolean): void {
  useEffect(() => {
    document.title = windowTitle(
      fileName === undefined ? undefined : { fileName, unsaved, ...(title === undefined ? {} : { title }) },
    );
  }, [fileName, title, unsaved]);
}

/** The registry with the reader's commands, and the context they run in: focus mode, the modes and the palette are the shell's. */
function useReaderCommands(
  platform: Platform,
  reader: ReaderController,
  viewer: Viewer | undefined,
  shell: Shell,
  rail: boolean,
  /** The handlers that need an open document: find, Back and Forward, print, download, properties, the selection's and the edits'. */
  documentActions: FindActions &
    Editing["actions"] &
    ReturnType<typeof useHistory>["actions"] &
    Output["actions"] &
    SelectionSurfaces["actions"] &
    ReaderMode["actions"],
  /** The handlers that work with or without a document: the settings'. */
  appActions: Settings["actions"],
  edits: Editing["edits"],
): { registry: CommandRegistry; context: CommandContext } {
  const { registry } = shell;
  // A mode registered later gets its `mode.<id>` handler: the context follows the modes.
  const modes = useStoreState(shell.modes, (s) => s.modes);
  const context = useMemo<CommandContext>(
    () => ({
      platform,
      reader,
      ...(viewer === undefined ? {} : { viewer }),
      ...(viewer === undefined || edits === undefined ? {} : { edits }),
      // Without rail tabs (no document open) the sidebar command has no handler and stays out of view.
      actions: {
        ...shellActions(shell.layout, shell.modes, { rail }),
        "palette.open": shell.openPalette,
        ...appActions,
        ...(viewer === undefined ? {} : documentActions),
        "privacy.network-activity": () => shell.layout.openInspector({ id: NETWORK_PANEL, selection: false }),
      },
    }),
    [platform, reader, viewer, shell, modes, rail, documentActions, appActions, edits],
  );
  return { registry, context };
}

/** The shell's parts every hook shares: the layout, the modes, the palette and the commands. */
interface Shell {
  readonly layout: ShellLayout;
  readonly modes: ModeRegistry;
  readonly openPalette: () => void;
  /** The commands, with the reader's registered; the edit commands join once a document is open. */
  readonly registry: CommandRegistry;
}

/** The shell's parts, made once, and whether the palette is open. */
function useAppShell(platform: Platform) {
  const [palette, setPalette] = useState(false);
  const { layout, modes } = useShell(platform.storage);
  const [shell] = useState<Shell>(() => ({
    layout,
    modes,
    openPalette: () => setPalette(true),
    registry: readerRegistry(),
  }));
  return { shell, palette, setPalette };
}

/** The commands' registry with the reader's commands. */
function readerRegistry(): CommandRegistry {
  const commands = new CommandRegistry();
  registerReaderCommands(commands);
  return commands;
}

/**
 * The status bar's items: the repaired notice, and under #debug the first page's timing and the
 * last edit's, from the call to its pages redrawn.
 */
function statusItems(
  engine: EngineState,
  debug: boolean,
  onRepairs: (() => void) | undefined,
  editMs: number | undefined,
): ReactNode[] {
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
  if (debug && editMs !== undefined) {
    items.push(<StatusBarItem key="edit-timing">{`last edit drawn in ${Math.round(editMs)} ms`}</StatusBarItem>);
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

/**
 * The inspector's panel, the status bar's items (reader mode's and the network indicator first, focus mode's
 * last) and the offline notice (R15).
 */
function usePanels(
  layout: ShellLayout,
  platform: Platform,
  network: NetworkMonitor,
  fileName: string | undefined,
  engine: EngineState,
  debug: boolean,
  shared: {
    readonly updates: AppUpdates;
    readonly toasts: ToastQueue;
    readonly documentOpen: boolean;
    readonly readerMode: ReaderMode;
    /** The last edit's time from the call to its pages redrawn, for #debug. */
    readonly editMs: number | undefined;
  },
): { inspector: ReactNode; status: ReactNode[]; notice: ReactNode } {
  const { inspector: repairsPanel, showRepairs } = useRepairsPanel(layout, platform, fileName, engine.repairs);
  const { indicator, inspector: networkPanel } = useNetworkPanel(layout, platform, network);
  const connection = useOffline(shared.updates, shared.toasts, shared.documentOpen);
  // One inspector at a time: whichever panel the layout holds draws, the others draw nothing.
  const inspector = shared.readerMode.inspector ?? repairsPanel ?? networkPanel;
  return {
    inspector,
    status: [
      ...(shared.readerMode.status === undefined ? [] : [shared.readerMode.status]),
      ...(indicator === undefined ? [] : [indicator]),
      ...statusItems(engine, debug, showRepairs, shared.editMs),
      ...connection.status,
      <FocusModeItem key="focus-mode" layout={layout} />,
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
  settings: Settings,
  shared: { readonly updates: AppUpdates; readonly toasts: ToastQueue },
  /** The edit session's handlers, which take Download a copy over while there are changes. */
  editActions: EditActions,
): {
  labels: readonly string[] | undefined;
  output: Output;
  moreItems: { items: ReturnType<typeof useGatedEntries<ReaderMoreId>>; onAction: (id: ReaderMoreId) => void };
  recent: Recent;
  /** The capability page (O05), over the reader at /capabilities. */
  capabilities: CapabilityRoute;
  /** The handlers that work with or without a document: the settings', recent files', install and update. */
  appActions: Settings["actions"];
} {
  const { updates, toasts } = shared;
  const appActions = settings.actions;
  const pages = useStoreState(engine, (s) => s.pages);
  const labels = useMemo(
    () => (pages.some((p) => p.label !== undefined) ? pages.map((_, i) => pageLabel(pages, i)) : undefined),
    [pages],
  );
  const document =
    state.phase === "open" && shown !== undefined
      ? { fileName: state.file.name, pages, labels, viewer: shown }
      : undefined;
  const output = useOutput(engine, platform, toasts, document);
  const recent = useRecent(platform, engine, state, settings.registry, toasts, () =>
    clearDocumentCache(platform.storage),
  );
  const install = useInstall();
  const capabilities = useCapabilityRoute(window);
  const items = useGatedEntries<ReaderMoreId>(moreEntries(document !== undefined, install.available));
  // The items that aren't one of the output's commands, each with what it does.
  const own: Record<OwnMoreId, () => void> = {
    open: () => void engine.reader.requestClose(),
    rotate: () => engine.getState().viewer?.rotate(90),
    settings: () => appActions["app.settings"]?.(undefined),
    shortcuts: () => appActions["help.shortcuts"]?.(undefined),
    capabilities: capabilities.show,
    install: () => install.actions["app.install"]?.(),
    privacy: () => void platform.openExternal(PRIVACY_URL).catch(() => undefined),
  };
  const onAction = (id: ReaderMoreId) => {
    if (id in own) {
      own[id as OwnMoreId]();
      return;
    }
    const command = MORE_COMMANDS[id as keyof typeof MORE_COMMANDS];
    (editActions[command as keyof EditActions] ?? output.actions[command])();
  };
  return {
    labels,
    output: { ...output, dialogs: [output.dialogs, install.dialog] },
    moreItems: { items, onAction },
    recent,
    capabilities,
    appActions: {
      ...appActions,
      "help.capabilities": capabilities.show,
      ...recent.actions,
      ...install.actions,
      ...(updates.getState().ready ? { "app.reload-update": () => updates.reload() } : {}),
    },
  };
}

/**
 * The More menu with the edit items first: they come with the edit commands, once a document is
 * open, and are made in the commands' own context.
 */
function withEditItems(
  edit: ReturnType<Editing["menu"]>,
  more: { readonly items: readonly MenuEntry<ReaderMoreId>[]; readonly onAction: (id: ReaderMoreId) => void },
): { items: MenuEntry<MoreId>[]; onAction: (id: MoreId) => void } {
  const isEditItem = (id: MoreId): id is EditMenuId => edit.items.some((item) => item.id === id);
  return {
    items: [...edit.items, ...more.items],
    onAction: (id) => (isEditItem(id) ? edit.run(id) : more.onAction(id)),
  };
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
  recent,
  readerMode,
}: {
  engine: Engine;
  state: ReaderState;
  platform: Platform;
  viewer: Viewer | undefined;
  /** The home screen's Recent list. */
  recent: ReactNode;
  /** Reader mode's column, over the covered view while it is on. */
  readerMode: ReaderMode;
}): ReactNode {
  const { reader } = engine;
  const linkProblem = useStoreState(engine, (s) => s.linkProblem);
  return (
    <>
      {state.phase === "empty" && linkProblem === undefined ? (
        <OpenDropZone reader={reader} keys={KEYS}>
          {recent}
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
      <ViewerHost viewer={viewer} visible={state.phase === "open"} covered={readerMode.on} />
      {state.phase === "open" ? readerMode.view : null}
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

/**
 * The rail's tabs for the open document, made again only when the view, the strip, find or the
 * History tab changes; History comes last, once an edit was made.
 */
function useRailTabs(
  engine: Engine,
  layout: ShellLayout,
  open: boolean,
  finding: boolean,
  history: RailTab | undefined,
) {
  const { strip, viewer } = useStoreState(engine, (s) => s);
  return useMemo(() => {
    const tabs = railTabs(engine, layout, open ? { strip, viewer, finding } : undefined);
    return tabs.length > 0 && history !== undefined ? [...tabs, history] : tabs;
  }, [engine, layout, open, strip, viewer, finding, history]);
}

/** The settings, with the page appearance of the document `viewer` shows, and the toasts they share. */
function useAppSettings(
  openPalette: () => void,
  viewer: Viewer | undefined,
  file: PlatformFile | undefined,
): { settings: Settings; toasts: ToastQueue } {
  const [toasts] = useState(() => new ToastQueue());
  return { settings: useSettings(openPalette, usePageAppearance(viewer, file, toasts)), toasts };
}

/**
 * What the chrome around the open document needs: find, the rail's tabs, Back and Forward, the
 * selection's surfaces, reader mode, and the commands with every handler they run.
 */
function useDocumentChrome(options: {
  readonly engine: Engine;
  readonly platform: Platform;
  readonly shell: Shell;
  readonly state: ReaderState;
  readonly shown: Viewer | undefined;
  readonly editing: Editing;
  readonly menu: { readonly output: Output; readonly appActions: Settings["actions"] };
}) {
  const { engine, platform, shell, state, shown, editing } = options;
  const { output, appActions } = options.menu;
  const open = state.phase === "open";
  const pages = useStoreState(engine, (s) => s.pages);
  const find = useFind(shown, open ? state.file : undefined);
  const tabs = useRailTabs(engine, shell.layout, open, find.open, editing.tab);
  const rail = tabs.length > 0;
  const history = useHistory(shown, pages);
  const selection = useSelection(shown, find, platform, KEYS);
  const reading = useReaderMode(engine, shell.layout, platform, shown, open ? state.file : undefined);
  const actions = {
    ...find.actions,
    ...history.actions,
    ...output.actions,
    ...selection.actions,
    ...reading.actions,
    ...editing.actions,
  };
  const commands = useReaderCommands(platform, engine.reader, shown, shell, rail, actions, appActions, editing.edits);
  return { find, tabs, rail, history, selection, reading, commands };
}

/** Two strips under the top bar, one over the other; either alone when the other is empty. */
function both(first: ReactNode, second: ReactNode): ReactNode {
  if (first === undefined || first === null) return second;
  if (second === undefined || second === null) return first;
  return (
    <>
      {first}
      {second}
    </>
  );
}

/** The reader over `engine`, on `platform`. */
export function App({ engine, platform, network, updates, debug }: AppProps): ReactNode {
  const state = useReaderState(engine.reader);
  const engineState = useStoreState(engine, (s) => s);
  const { shell, palette, setPalette } = useAppShell(platform);
  const { layout, modes } = shell;
  const sidebar = useStoreState(layout, (s) => s.rail.open);
  const open = state.phase === "open";
  const shown = open ? engineState.viewer : undefined;
  const { settings, toasts } = useAppSettings(shell.openPalette, shown, open ? state.file : undefined);
  const fileName = open ? state.file.name : undefined;
  const editing = useEditing({ engine, shell, viewer: shown, toasts, fileName, keys: KEYS, platform });
  const menu = useDocumentMenu(engine, platform, state, shown, settings, { updates, toasts }, editing.actions);
  const { labels, output, recent } = menu;
  const chrome = useDocumentChrome({ engine, platform, shell, state, shown, editing, menu });
  const { find, tabs, rail, history, selection, reading, commands } = chrome;
  const moreItems = withEditItems(editing.menu(commands.context), menu.moreItems);
  useWindowTitle(fileName, engineState.title, editing.unsaved);
  const liveRegions = useAnnouncements(shown, open ? state.file : undefined, engineState);
  const { inspector, status, notice } = usePanels(layout, platform, network, fileName, engineState, debug, {
    updates,
    toasts,
    documentOpen: open,
    readerMode: reading,
    editMs: editing.lastMs,
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
        notice={both(notice, editing.notice)}
        topBar={
          <ReaderTopBar
            reader={engine.reader}
            {...(shown === undefined ? {} : { viewer: shown })}
            editArea={editing.topBar}
            keys={KEYS}
            sidebarOpen={sidebar}
            findOpen={find.open}
            {...reading.topBar}
            {...(shown === undefined ? {} : { history: history.history })}
            {...(labels === undefined ? {} : { labels })}
            home={<span className="web-wordmark">{APP_NAME}</span>}
            debug={debug ? <EngineDebugMenu engine={engine} backend={engineState.backend} /> : undefined}
            view={settings.viewButton}
            more={moreItems}
          />
        }
        rail={<LeftRail layout={layout} tabs={tabs} />}
        bottomBar={selection.bar ?? <PhoneBottomBar layout={layout} rail={rail} more={moreItems} keys={KEYS} />}
        {...(inspector === undefined ? {} : { inspector })}
        status={status}
      >
        <Canvas
          engine={engine}
          state={state}
          platform={platform}
          viewer={engineState.viewer}
          recent={recent.home}
          readerMode={reading}
        />
        <ReaderDialogs engine={engine} link={engineState.externalLink} platform={platform} />
        {/* The link previews (R07) load once a document is open (later.ts, #380). */}
        {shown === undefined || editing.later === undefined ? null : (
          <editing.later.LinkPreviews engine={engine} viewer={shown} />
        )}
        {output.dialogs}
        {editing.runtime(commands.registry)}
      </ReaderShell>
      <ToastRegion queue={toasts} aria-label={message("shell.toasts")} />
      <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
      {settings.dialogs}
      {menu.capabilities.render(fileName)}
      {liveRegions}
      {selection.toolbar}
    </CommandsProvider>
  );
}
