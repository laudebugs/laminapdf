// The desktop reader's window: the reader's chrome around `@laminapdf/viewer`, which draws tiles
// from this window's engine process through `DesktopEngineClient` (ADR 0008). The Open button
// runs the main process's file dialog; a locked document asks for its password; a restarted
// engine shows "Document process restarted" and the view redraws by itself. Files the OS asks the
// app to open (the command line, a second launch, Finder, the Dock, a drop on the window) open in
// a new tab (tabs/), and a file opened again shows its tab. The start window lists recent files; a
// document whose file changes on disk reloads in place (file-changes.tsx); Settings holds the Files
// choices.

import type { KeyPlatform, Platform, ReaderError, ReaderState } from "@laminapdf/ui";
import {
  type CommandContext,
  type CommandRegistry,
  CommandsProvider,
  DropOverlay,
  type EditControls,
  FocusModeItem,
  message,
  Notice,
  OpenDropZone,
  type Opener,
  OpenFailure,
  PasswordDialog,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  RegistryPalette,
  RepairedNotice,
  registerDesktopCommands,
  registerReaderCommands,
  type ShellLayout,
  ShortcutsDialog,
  StatusBarItem,
  ToastQueue,
  ToastRegion,
  useStoreState,
} from "@laminapdf/ui";
import { registerEditCommands } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AppInfo } from "./app-info.ts";
import { type CrashSetting, useCrashReports, useCrashSetting } from "./crashes.tsx";
import { useDesktopEditing } from "./editing.tsx";
import { type DesktopEngineClient, DesktopEngineError, type DesktopShell } from "./engine/desktop-engine-client.ts";
import { type Dragging, desktopOpener, listenForOsOpens } from "./engine/open.ts";
import { stopper } from "./engine/stopper.ts";
import { MissingRestoredTab, useFileChanges } from "./file-changes.tsx";
import { useDesktopHistory } from "./history.ts";
import { EchoRegistry } from "./menus/echo-registry.ts";
import { DESKTOP_EDIT_COMMANDS } from "./menus/menus.ts";
import { useNativeMenus } from "./menus/use-native-menus.tsx";
import { useDesktopNetwork } from "./network.tsx";
import { DesktopOrganiseMode, useOrganising } from "./organise.tsx";
import { NotYetSupportedError } from "./platform/desktop.ts";
import { useOutput } from "./properties.tsx";
import { type DesktopRecent, useDesktopRecent } from "./recent.tsx";
import { useDesktopSaving } from "./saving.tsx";
import { type DesktopSettings, useDesktopSettings } from "./settings.tsx";
import { NO_TABS, useTabStrip } from "./tabs/use-tabs.tsx";
import { WindowTabs } from "./tabs/window-tabs.ts";
import { type DesktopUpdates, UpdateReadyItem, updateActions, useDesktopUpdates } from "./updates.tsx";

/** Outside Tauri (`pnpm dev` in a browser) there is no engine to open with. */
const openLater: Opener = () => Promise.reject(new NotYetSupportedError("open"));

/** This window's engine: the shell, the client over it and the view it draws. */
export interface DesktopEngine {
  readonly shell: DesktopShell;
  readonly client: DesktopEngineClient;
  readonly viewer: Viewer;
}

/** Props of `App`. */
export interface AppProps {
  readonly platform: Platform;
  /** The shell's own report, or undefined while it loads. */
  readonly info: AppInfo | undefined;
  /** How shortcuts are written and which key `Mod` means; the entry decides. */
  readonly keys: KeyPlatform;
  /** The engine, or undefined outside the desktop shell. */
  readonly engine: DesktopEngine | undefined;
}

/** Why the open document can't be shown any more, after its engine restarted. */
type Lost = "failed" | "closed" | "locked";

const LOST_TEXT: Readonly<Record<Lost, { title: string; body: string }>> = {
  failed: {
    title: "The document process stopped",
    body: "It crashed several times in a row, so Lamina PDF stopped restarting it. Reopen the document to try again.",
  },
  closed: {
    title: "This document couldn't be reopened",
    body: "Its document process restarted, but the file didn't open again. Reopen it to try again.",
  },
  locked: {
    title: "This document needs its password again",
    body: "Its document process restarted and the kept password was refused. Reopen it and type the password.",
  },
};

/** A part of the window's command handlers. */
type CommandActions = NonNullable<CommandContext["actions"]>;

/** The window's command handlers in one object, the same object while each part is. */
function useMergedActions(
  home: CommandActions,
  tabs: CommandActions,
  print: CommandActions,
  edits: CommandActions,
  saves: CommandActions,
  network: CommandActions,
  history: CommandActions,
  organise: CommandActions,
): CommandActions {
  // Organise's page commands come last: while it is the mode, they act on the grid's selection.
  return useMemo(
    () => ({ ...home, ...tabs, ...print, ...edits, ...saves, ...network, ...history, ...organise }),
    [home, tabs, print, edits, saves, network, history, organise],
  );
}

/** {@link WindowStatus}'s props. */
interface WindowStatusProps {
  /** The network activity indicator. */
  readonly network: ReactNode;
  /** Whether the shown document was repaired as it opened. */
  readonly repaired: boolean;
  /** Where the repair's details go. */
  readonly toasts: ToastQueue;
  /** The print's progress, while one runs. */
  readonly print: ReactNode;
  /** The app's updates, for "Update ready". */
  readonly updates: DesktopUpdates;
  /** The window's layout store, for focus mode's status-bar toggle and Escape layer. */
  readonly layout: ShellLayout;
  /** The app's version, when known. */
  readonly info: AppInfo | undefined;
}

/**
 * The status bar's items: network activity, the repair notice, the print's progress, "Update ready",
 * "Focus mode" and the app's version.
 * @param props - See {@link WindowStatusProps}.
 * @returns The items.
 */
function WindowStatus({ network, repaired, toasts, print, updates, layout, info }: WindowStatusProps): ReactNode {
  return (
    <>
      {network}
      {repaired ? (
        <RepairedNotice
          label="Repaired on open"
          details="Details"
          onPress={() =>
            toasts.add({
              message: "The file had damage that Lamina PDF repaired as it opened it. The file itself is unchanged.",
            })
          }
        />
      ) : null}
      {print}
      <UpdateReadyItem updates={updates} />
      <FocusModeItem layout={layout} />
      {info === undefined ? null : <StatusBarItem>{`Lamina PDF ${info.version}`}</StatusBarItem>}
    </>
  );
}

/** Why the document's view went away, with Reopen. */
function LostNotice({ lost, reader }: { lost: Lost; reader: ReaderController }): ReactNode {
  return (
    <Notice title={LOST_TEXT[lost].title} tone="warning">
      {LOST_TEXT[lost].body}{" "}
      <button type="button" className="desktop-link" onClick={() => void reopen(reader)}>
        Reopen
      </button>
    </Notice>
  );
}

/** Creates this window's tabs store (when multi-window is supported), reader controller and rename callback. */
function useWindowReader(
  platform: Platform,
  engine: DesktopEngine | undefined,
): { tabs: WindowTabs | undefined; reader: ReaderController; renamed: (doc: number, name: string) => void } {
  // The tabs come first: the reader's opener tells them which document shows.
  const [tabs] = useState(() =>
    engine === undefined || !platform.capabilities.multiWindow
      ? undefined
      : new WindowTabs(engine.shell, engine.client, engine.viewer),
  );
  const [reader] = useState(
    () =>
      new ReaderController(
        platform,
        engine === undefined ? openLater : desktopOpener(engine.shell, engine.client, engine.viewer, tabs),
      ),
  );
  const renamed = useCallback(
    (doc: number, name: string) => {
      tabs?.renamed(doc, name);
      reader.fileRenamed();
    },
    [tabs, reader],
  );
  return { tabs, reader, renamed };
}

/** Opens this window's reader, tabs, toasts and the stores that follow the document, saving, editing and home screen. */
function useWindowStores(platform: Platform, keys: KeyPlatform, engine: DesktopEngine | undefined) {
  const { tabs, reader, renamed } = useWindowReader(platform, engine);
  const state = useStoreState(reader, (s) => s);
  const [toasts] = useState(() => new ToastQueue());
  const lost = useEngineEvents(engine, toasts, state.phase === "open");
  const dragging = useOsOpens(engine, reader, toasts, tabs);
  const fileProblem = useFileChanges(engine, reader, toasts, tabs);
  const saving = useDesktopSaving(engine, reader, toasts, renamed);
  const editing = useDesktopEditing(engine, reader, engine?.client.current?.name, saving.save);
  const strip = useTabStrip(engine?.shell, tabs, reader, toasts, keys, engine?.viewer, editing.unsaved);
  const home = useHome(engine, platform, reader, state, toasts);
  const output = useOutput(engine, platform, toasts);
  const crashes = useCrashReports(engine?.shell);
  return { tabs, reader, state, toasts, lost, dragging, fileProblem, saving, editing, strip, home, output, crashes };
}

/** Hooks up the window's document, tab, edit, settings, network and command state. */
function useDesktopWindow(platform: Platform, keys: KeyPlatform, engine: DesktopEngine | undefined) {
  const { tabs, reader, state, toasts, lost, dragging, fileProblem, saving, editing, strip, home, output, crashes } =
    useWindowStores(platform, keys, engine);
  const open = state.phase === "open";
  const viewer = open && lost === undefined ? engine?.viewer : undefined;
  const navigation = useDesktopHistory(viewer);
  const { actions: paging, ...organising } = useOrganising(platform, engine, open && lost === undefined, toasts);
  const network = useDesktopNetwork(
    engine?.shell,
    organising.shell.layout,
    platform,
    home.updates.status,
    home.crashSetting,
    toasts,
  );
  const actions = useMergedActions(
    home.actions,
    strip.actions,
    output.actions,
    editing.actions,
    saving.actions,
    network.actions,
    navigation.actions,
    paging,
  );
  const shows = useShownParts(state.phase, tabs);
  useKeepFocus(organising.shell.layout, shows, state.phase, lost);
  const commands = useCommands({
    platform,
    reader,
    actions,
    settings: home.settings,
    viewer,
    edits: editing.edits,
    layout: organising.shell.layout,
  });
  useNativeMenus(engine?.shell, commands.registry, commands.context);
  const repaired = open && engine?.client.current?.repaired === true;
  return {
    tabs,
    reader,
    state,
    toasts,
    lost,
    dragging,
    fileProblem,
    dialogs: [editing.dialog, output.dialog, saving.dialog, home.settings.dialog, crashes.dialog, commands.palette],
    strip,
    home,
    output,
    crashes,
    open,
    organising,
    network,
    shows,
    viewer,
    history: navigation.history,
    commands,
    repaired,
  };
}

/** Leaves focus mode once the window no longer shows a document (and is not mid-switch between tabs). */
function useKeepFocus(
  layout: ShellLayout,
  shows: { readonly viewer: boolean; readonly hasTabs: boolean },
  phase: ReaderState["phase"],
  lost: Lost | undefined,
): void {
  const keepFocus = (shows.viewer || (phase === "empty" && shows.hasTabs)) && lost === undefined;
  useEffect(() => {
    if (!keepFocus) layout.leaveFocus();
  }, [keepFocus, layout]);
}

/**
 * The reader's window: top bar, home screen, the document's view, and the notices around them.
 * @param props - See {@link AppProps}.
 * @returns The window's content.
 */
export function App({ platform, info, keys, engine }: AppProps): ReactNode {
  const w = useDesktopWindow(platform, keys, engine);
  return (
    <CommandsProvider
      registry={w.commands.registry}
      context={w.commands.context}
      keys={keys}
      singleKeys={w.home.settings.singleKeys}
    >
      <ReaderShell
        {...w.organising.shell}
        inspector={w.network.inspector}
        tabs={w.strip.strip}
        notice={w.crashes.notice ?? w.strip.notice}
        topBar={
          <DesktopTopBar
            reader={w.reader}
            viewer={w.viewer}
            history={w.history}
            keys={keys}
            organising={w.organising.topBar}
            settings={w.home.settings}
          />
        }
        status={
          <WindowStatus
            network={w.network.indicator}
            repaired={w.repaired}
            toasts={w.toasts}
            print={w.output.status}
            updates={w.home.updates}
            layout={w.organising.shell.layout}
            info={info}
          />
        }
      >
        {w.shows.startScreen ? (
          <OpenDropZone reader={w.reader} keys={keys}>
            {w.home.recent.home}
          </OpenDropZone>
        ) : null}
        {w.open && w.lost === undefined ? w.fileProblem : null}
        {w.state.phase === "error" ? (
          <ErrorPage
            state={w.state}
            engine={engine}
            tabs={w.tabs}
            reader={w.reader}
            platform={platform}
            toasts={w.toasts}
          />
        ) : null}
        {w.open && w.lost !== undefined ? <LostNotice lost={w.lost} reader={w.reader} /> : null}
        <ViewerHost viewer={engine?.viewer} visible={w.shows.viewer && w.lost === undefined} />
        <DesktopOrganiseMode {...w.organising.mode} viewer={w.viewer} registry={w.commands.registry} keys={keys} />
      </ReaderShell>
      <WindowOverlays dragging={w.dragging} reader={w.reader} toasts={w.toasts}>
        {w.dialogs}
      </WindowOverlays>
    </CommandsProvider>
  );
}

/** The window's top bar: document controls, View settings popover and More › Settings. */
function DesktopTopBar({
  reader,
  viewer,
  history,
  keys,
  organising,
  settings,
}: {
  reader: ReaderController;
  viewer: Viewer | undefined;
  history: { readonly back: string | undefined; readonly forward: string | undefined } | undefined;
  keys: KeyPlatform;
  organising: ReturnType<typeof useOrganising>["topBar"];
  settings: DesktopSettings;
}): ReactNode {
  return (
    <ReaderTopBar
      reader={reader}
      {...(viewer === undefined ? {} : { viewer })}
      {...(history === undefined ? {} : { history })}
      keys={keys}
      {...organising}
      home={<span className="desktop-wordmark">Lamina PDF</span>}
      view={settings.viewButton}
      more={{
        items: [{ id: "settings", label: message("command.app.settings"), icon: "settings" }],
        onAction: settings.open,
      }}
    />
  );
}

/** The window's drop overlay, dialogs and toast region. */
function WindowOverlays({
  dragging,
  reader,
  children,
  toasts,
}: {
  dragging: Dragging | undefined;
  reader: ReaderController;
  children: ReactNode;
  toasts: ToastQueue;
}): ReactNode {
  return (
    <>
      {dragging === undefined ? null : <DropOverlay>{dropText(dragging)}</DropOverlay>}
      <PasswordDialog reader={reader} />
      {children}
      <ToastRegion queue={toasts} aria-label="Notifications" />
    </>
  );
}

/**
 * The reader's commands (open, the shortcuts) and the context they run in, with the command palette
 * (⌘K or Ctrl+K) as in the web reader: it reaches every command with a handler here, among them
 * Open recent and Clear recent while a document is open. Help ▸ Keyboard shortcuts (F1) shows the
 * shortcut help, which lists the same keys as the native menus.
 */
function useCommands({
  platform,
  reader,
  actions,
  settings,
  viewer,
  edits,
  layout,
}: {
  platform: Platform;
  reader: ReaderController;
  actions: CommandContext["actions"];
  settings: DesktopSettings;
  viewer: Viewer | undefined;
  edits: EditControls | undefined;
  layout: ShellLayout;
}): { registry: CommandRegistry; context: CommandContext; palette: ReactNode } {
  const [registry] = useState(() => {
    // The menus' accelerators and the web view's shortcuts can both see one key press.
    const commands = new EchoRegistry();
    registerReaderCommands(commands);
    registerDesktopCommands(commands);
    registerEditCommands(commands, DESKTOP_EDIT_COMMANDS);
    return commands;
  });
  const [isOpen, setOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const { checked, singleKeys, setSingleKeys } = settings;
  const context = useMemo<CommandContext>(
    () => ({
      platform,
      reader,
      layout,
      actions: {
        ...actions,
        "palette.open": () => setOpen(true),
        "help.shortcuts": () => setShortcutsOpen(true),
      },
      checked,
      ...(viewer === undefined ? {} : { viewer }),
      ...(viewer === undefined || edits === undefined ? {} : { edits }),
    }),
    [platform, reader, layout, actions, checked, viewer, edits],
  );
  return {
    registry,
    context,
    palette: (
      <>
        <RegistryPalette isOpen={isOpen} onOpenChange={setOpen} />
        <ShortcutsDialog
          isOpen={shortcutsOpen}
          onOpenChange={setShortcutsOpen}
          singleKeys={singleKeys}
          onSingleKeysChange={setSingleKeys}
          onOpenPalette={() => setOpen(true)}
        />
      </>
    ),
  };
}

/**
 * The start window's recent files and the Settings dialog, which share the "Remember recent
 * files" choice, the updates (S06), and the handlers of their commands (`file.open-recent`,
 * `file.clear-recent`, `app.settings`, `app.check-for-updates`, `app.restart-to-update`).
 */
function useHome(
  engine: DesktopEngine | undefined,
  platform: Platform,
  reader: ReaderController,
  state: ReaderState,
  toasts: ToastQueue,
): {
  settings: DesktopSettings;
  recent: DesktopRecent;
  updates: DesktopUpdates;
  crashSetting: CrashSetting | undefined;
  actions: NonNullable<CommandContext["actions"]>;
} {
  // The recent list reads again once a preference change has reached the main process.
  const [preferencesChanged, setPreferencesChanged] = useState(0);
  const updates = useDesktopUpdates(engine?.shell, toasts);
  const crashReports = useCrashSetting(engine?.shell);
  const settings = useDesktopSettings(
    engine?.shell,
    platform.storage,
    useCallback(() => setPreferencesChanged((n) => n + 1), []),
    updates,
    crashReports,
    toasts,
    engine?.viewer,
  );
  const recent = useDesktopRecent(platform, reader, state, toasts, settings.preferences.rememberRecent);
  const { refresh, actions: recentActions } = recent;
  useEffect(() => {
    if (preferencesChanged > 0) refresh();
  }, [preferencesChanged, refresh]);
  const openSettings = settings.open;
  const settingsActions = settings.actions;
  const inShell = engine !== undefined;
  const { status } = updates;
  const { checkNow, restart } = updates;
  const actions = useMemo(
    () => ({
      ...recentActions,
      ...settingsActions,
      "app.settings": () => openSettings(),
      ...updateActions(inShell, { status, checkNow, restart }, openSettings),
    }),
    [recentActions, settingsActions, openSettings, inShell, status, checkNow, restart],
  );
  return { settings, recent, updates, crashSetting: crashReports[0], actions };
}

/** What a drop of the dragged files will do. */
function dropText({ pdfs }: Dragging): string {
  if (pdfs === 0) return message("reader.open.droppingNone");
  return pdfs === 1 ? message("reader.open.dropping") : message("reader.open.droppingMany", { count: pdfs });
}

/**
 * Opens what the OS asks this window to open, in place of what it shows, and says when a file
 * couldn't be read or a drop held no PDF. Returns what is being dragged over the window, if anything.
 */
function useOsOpens(
  engine: DesktopEngine | undefined,
  reader: ReaderController,
  toasts: ToastQueue,
  tabs: WindowTabs | undefined,
): Dragging | undefined {
  const [dragging, setDragging] = useState<Dragging>();
  useEffect(() => {
    if (engine === undefined) return;
    return stopper(
      listenForOsOpens(engine.shell, {
        // After the last session's tabs, so a file named at launch opens last and shows.
        open: async (file) => {
          await tabs?.whenRestored();
          await reader.open(file);
        },
        unreadable: (name) => toasts.add({ message: message("reader.open.missing", { name }) }),
        dragging: setDragging,
        refused: (names) => toasts.add({ message: `${message("reader.open.droppingNone")}: ${names.join(", ")}` }),
      }),
      "listening for the OS's open requests",
    );
  }, [engine, reader, toasts, tabs]);
  return dragging;
}

/** Closes the lost document and asks for it again through the open dialog. */
async function reopen(reader: ReaderController): Promise<void> {
  reader.close();
  await reader.openFromPicker();
}

/**
 * Listens to this window's engine: a restart that kept the document shows a toast (the viewer
 * redraws by itself); one that lost it, or a failed engine, returns why.
 */
function useEngineEvents(engine: DesktopEngine | undefined, toasts: ToastQueue, open: boolean): Lost | undefined {
  const [lost, setLost] = useState<Lost>();
  useEffect(() => {
    if (engine === undefined) return;
    const off = [
      engine.client.on("restarted", ({ reopened }) => {
        if (reopened) toasts.add({ message: "Document process restarted" });
      }),
      engine.client.on("password-required", () => setLost("locked")),
      engine.client.on("restart-failed", ({ error }) =>
        setLost(error instanceof DesktopEngineError && error.code === "closed" ? "closed" : "failed"),
      ),
    ];
    return () => {
      for (const unsubscribe of off) unsubscribe();
    };
  }, [engine, toasts]);
  // A newly opened document starts afresh.
  useEffect(() => {
    if (!open) setLost(undefined);
  }, [open]);
  return lost;
}

/**
 * What the window shows when the reader couldn't open its document: a tab of the last session
 * whose file is gone offers Locate…; anything else gets the reader's failure page.
 */
function ErrorPage({
  state,
  engine,
  tabs,
  reader,
  platform,
  toasts,
}: {
  state: Extract<ReaderState, { phase: "error" }>;
  engine: DesktopEngine | undefined;
  tabs: WindowTabs | undefined;
  reader: ReaderController;
  platform: Platform;
  toasts: ToastQueue;
}): ReactNode {
  // A restored tab whose file is gone; any other failure (a damaged file) gets the failure page.
  if (engine !== undefined && state.error.code === "io" && tabs?.isUnopened(state.file) === true) {
    return <MissingRestoredTab engine={engine} tabs={tabs} file={state.file} toasts={toasts} />;
  }
  return <Failure error={state.error} reader={reader} platform={platform} />;
}

/** The page that says why a document didn't open, with the reader's own failure layout. */
function Failure({
  error,
  reader,
  platform,
}: {
  error: ReaderError;
  reader: ReaderController;
  platform: Platform;
}): ReactNode {
  return (
    <OpenFailure
      error={error}
      onTryAnother={() => reader.close()}
      onCopy={() => void platform.copyText(`${error.code}: ${error.message}`)}
    />
  );
}

/**
 * Whether the window shows its start screen and its viewer. With tabs open, a document that is
 * opening (a tab shown, a file opened) replaces the one on screen: the viewer stays and the start
 * screen doesn't flash. A hide and show within one frame would also leave the viewer without the
 * size it waits for.
 */
function useShownParts(
  phase: ReaderState["phase"],
  tabs: WindowTabs | undefined,
): { startScreen: boolean; viewer: boolean; hasTabs: boolean } {
  const hasTabs = useStoreState(tabs ?? NO_TABS, (s) => s.tabs.length > 0);
  const between = phase === "opening" && hasTabs;
  return {
    startScreen: phase === "empty" || phase === "needs-password" || (phase === "opening" && !hasTabs),
    viewer: phase === "open" || between,
    hasTabs,
  };
}

/** Mounts `viewer` into a box that fills the canvas, and shows it while `visible`. */
function ViewerHost({ viewer, visible }: { viewer: Viewer | undefined; visible: boolean }): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (viewer !== undefined && host.current !== null) viewer.mount(host.current);
  }, [viewer]);
  return <div ref={host} className="desktop-viewer" hidden={!visible} />;
}
