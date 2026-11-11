// The desktop reader's window: the reader's chrome around `@laminapdf/viewer`, which draws tiles
// from this window's engine process through `DesktopEngineClient` (ADR 0008). The Open button
// runs the main process's file dialog; a locked document asks for its password; a restarted
// engine shows "Document process restarted" and the view redraws by itself.

import type { KeyPlatform, Platform, ReaderError } from "@laminapdf/ui";
import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  Notice,
  OpenDropZone,
  type Opener,
  OpenFailure,
  PasswordDialog,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  RepairedNotice,
  registerReaderCommands,
  StatusBarItem,
  ToastQueue,
  ToastRegion,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import type { AppInfo } from "./app-info.ts";
import type { DesktopEngineClient, DesktopShell } from "./engine/desktop-engine-client.ts";
import { desktopOpener } from "./engine/open.ts";
import { NotYetSupportedError } from "./platform/desktop.ts";

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
  // The reader's commands (open, the shortcuts); the shell adds its own handlers as it grows.
  const [registry] = useState(() => {
    const commands = new CommandRegistry();
    registerReaderCommands(commands);
    return commands;
  });
  const context = useMemo<CommandContext>(() => ({ platform, reader }), [platform, reader]);
  const open = state.phase === "open";
  const repaired = open && engine?.client.current?.repaired === true;
  return (
    <CommandsProvider registry={registry} context={context} keys={keys}>
      <ReaderShell
        topBar={
          <ReaderTopBar
            reader={reader}
            keys={keys}
            home={<span className="desktop-wordmark">Lamina PDF</span>}
            more={{ items: [], onAction: () => undefined }}
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
          <OpenDropZone reader={reader} keys={keys} />
        ) : null}
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
      <PasswordDialog reader={reader} />
      <ToastRegion queue={toasts} aria-label="Notifications" />
    </CommandsProvider>
  );
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
        setLost(
          typeof error === "object" && error !== null && "code" in error && error.code === "closed"
            ? "closed"
            : "failed",
        ),
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
