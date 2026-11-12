// The desktop reader's window: the reader's chrome around `@laminapdf/viewer`, which draws tiles
// from this window's engine process through `DesktopEngineClient` (ADR 0008). The Open button
// runs the main process's file dialog; a locked document asks for its password; a restarted
// engine shows "Document process restarted" and the view redraws by itself. Files the OS asks the
// app to open (the command line, a second launch, Finder, the Dock, a drop on the window) replace
// what the window shows, until it has tabs (D4a). The start window lists recent files; a document
// whose file changes on disk reloads in place (file-changes.tsx); Settings holds the Files choices.

import type { KeyPlatform, Platform, ReaderError, ReaderState } from "@laminapdf/ui";
import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  DropOverlay,
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
  registerReaderCommands,
  StatusBarItem,
  ToastQueue,
  ToastRegion,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AppInfo } from "./app-info.ts";
import { type DesktopEngineClient, DesktopEngineError, type DesktopShell } from "./engine/desktop-engine-client.ts";
import { type Dragging, desktopOpener, listenForOsOpens } from "./engine/open.ts";
import { stopper } from "./engine/stopper.ts";
import { useFileChanges } from "./file-changes.tsx";
import { NotYetSupportedError } from "./platform/desktop.ts";
import { type DesktopRecent, useDesktopRecent } from "./recent.tsx";
import { type DesktopSettings, useDesktopSettings } from "./settings.tsx";

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

/**
 * The reader's window: top bar, home screen, the document's view, and the notices around them.
 * @param props - See {@link AppProps}.
 * @returns The window's content.
 */
export function App({ platform, info, keys, engine }: AppProps): ReactNode {
  const [reader] = useState(
    () =>
      new ReaderController(
        platform,
        engine === undefined ? openLater : desktopOpener(engine.shell, engine.client, engine.viewer),
      ),
  );
  const state = useStoreState(reader, (s) => s);
  const [toasts] = useState(() => new ToastQueue());
  const lost = useEngineEvents(engine, toasts, state.phase === "open");
  const dragging = useOsOpens(engine, reader, toasts);
  const fileProblem = useFileChanges(engine, reader, toasts);
  const { settings, recent, actions } = useHome(engine, platform, reader, state, toasts);
  const open = state.phase === "open";
  // The page input, zoom and the viewer's commands work while a document is on screen.
  const viewer = open && lost === undefined ? engine?.viewer : undefined;
  const { registry, context, palette } = useCommands(platform, reader, actions, viewer);
  const repaired = open && engine?.client.current?.repaired === true;
  return (
    <CommandsProvider registry={registry} context={context} keys={keys}>
      <ReaderShell
        topBar={
          <ReaderTopBar
            reader={reader}
            {...(viewer === undefined ? {} : { viewer })}
            keys={keys}
            home={<span className="desktop-wordmark">Lamina PDF</span>}
            more={{
              items: [{ id: "settings", label: message("command.app.settings"), icon: "settings" }],
              onAction: settings.open,
            }}
          />
        }
        status={
          <>
            {repaired ? (
              <RepairedNotice
                label="Repaired on open"
                details="Details"
                onPress={() =>
                  toasts.add({
                    message:
                      "The file had damage that Lamina PDF repaired as it opened it. The file itself is unchanged.",
                  })
                }
              />
            ) : null}
            {info === undefined ? null : <StatusBarItem>{`Lamina PDF ${info.version}`}</StatusBarItem>}
          </>
        }
      >
        {state.phase === "empty" || state.phase === "opening" || state.phase === "needs-password" ? (
          <OpenDropZone reader={reader} keys={keys}>
            {recent.home}
          </OpenDropZone>
        ) : null}
        {open && lost === undefined ? fileProblem : null}
        {state.phase === "error" ? <Failure error={state.error} reader={reader} platform={platform} /> : null}
        {open && lost !== undefined ? (
          <Notice title={LOST_TEXT[lost].title} tone="warning">
            {LOST_TEXT[lost].body}{" "}
            <button type="button" className="desktop-link" onClick={() => void reopen(reader)}>
              Reopen
            </button>
          </Notice>
        ) : null}
        <ViewerHost viewer={engine?.viewer} visible={open && lost === undefined} />
      </ReaderShell>
      {dragging === undefined ? null : <DropOverlay>{dropText(dragging)}</DropOverlay>}
      <PasswordDialog reader={reader} />
      {settings.dialog}
      {palette}
      <ToastRegion queue={toasts} aria-label="Notifications" />
    </CommandsProvider>
  );
}

/**
 * The reader's commands (open, the shortcuts) and the context they run in, with the command palette
 * (⌘K or Ctrl+K) as in the web reader: it reaches every command with a handler here, among them
 * Open recent and Clear recent while a document is open.
 */
function useCommands(
  platform: Platform,
  reader: ReaderController,
  actions: CommandContext["actions"],
  viewer: Viewer | undefined,
): { registry: CommandRegistry; context: CommandContext; palette: ReactNode } {
  const [registry] = useState(() => {
    const commands = new CommandRegistry();
    registerReaderCommands(commands);
    return commands;
  });
  const [isOpen, setOpen] = useState(false);
  const context = useMemo<CommandContext>(
    () => ({
      platform,
      reader,
      actions: { ...actions, "palette.open": () => setOpen(true) },
      ...(viewer === undefined ? {} : { viewer }),
    }),
    [platform, reader, actions, viewer],
  );
  return { registry, context, palette: <RegistryPalette isOpen={isOpen} onOpenChange={setOpen} /> };
}

/**
 * The start window's recent files and the Settings dialog, which share the "Remember recent
 * files" choice, and the handlers of their commands (`file.open-recent`, `file.clear-recent`,
 * `app.settings`).
 */
function useHome(
  engine: DesktopEngine | undefined,
  platform: Platform,
  reader: ReaderController,
  state: ReaderState,
  toasts: ToastQueue,
): { settings: DesktopSettings; recent: DesktopRecent; actions: NonNullable<CommandContext["actions"]> } {
  // The recent list reads again once a preference change has reached the main process.
  const [preferencesChanged, setPreferencesChanged] = useState(0);
  const settings = useDesktopSettings(
    engine?.shell,
    useCallback(() => setPreferencesChanged((n) => n + 1), []),
  );
  const recent = useDesktopRecent(platform, reader, state, toasts, settings.preferences.rememberRecent);
  const { refresh, actions: recentActions } = recent;
  useEffect(() => {
    if (preferencesChanged > 0) refresh();
  }, [preferencesChanged, refresh]);
  const openSettings = settings.open;
  const actions = useMemo(() => ({ ...recentActions, "app.settings": openSettings }), [recentActions, openSettings]);
  return { settings, recent, actions };
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
): Dragging | undefined {
  const [dragging, setDragging] = useState<Dragging>();
  useEffect(() => {
    if (engine === undefined) return;
    return stopper(
      listenForOsOpens(engine.shell, {
        open: (file) => reader.open(file),
        unreadable: (name) => toasts.add({ message: message("reader.open.missing", { name }) }),
        dragging: setDragging,
        refused: (names) => toasts.add({ message: `${message("reader.open.droppingNone")}: ${names.join(", ")}` }),
      }),
      "listening for the OS's open requests",
    );
  }, [engine, reader, toasts]);
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

/** Mounts `viewer` into a box that fills the canvas, and shows it while `visible`. */
function ViewerHost({ viewer, visible }: { viewer: Viewer | undefined; visible: boolean }): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (viewer !== undefined && host.current !== null) viewer.mount(host.current);
  }, [viewer]);
  return <div ref={host} className="desktop-viewer" hidden={!visible} />;
}
