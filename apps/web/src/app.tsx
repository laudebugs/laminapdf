// The reader: @laminapdf/ui's chrome around @laminapdf/viewer's view. The home screen opens a file
// from Choose a file, a drop anywhere in the window or a paste; the canvas then shows the document,
// or the failure page when it can't be opened. A repaired file gets the status bar's notice and
// the repair details panel, docked as the shell's inspector. Focus mode (Ctrl+\ or the palette) hides
// the secondary toolbar; the rail arrives with its tabs (thumbnails and the outline). #debug adds the
// backend switch and the first page's timing.

import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  DebugMenu,
  FocusModeItem,
  Inspector,
  ModeRegistry,
  message,
  OpenDropZone,
  OpenFailure,
  PasswordDialog,
  type Platform,
  type ReaderController,
  ReaderShell,
  ReaderTopBar,
  RegistryPalette,
  RepairDetails,
  RepairedNotice,
  registerReaderCommands,
  ShellLayout,
  StatusBarItem,
  shellActions,
  useGatedEntries,
  useReaderState,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { APP_NAME, repairsText, windowTitle } from "./document.ts";
import type { Engine, EngineState } from "./session.ts";

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

/** The shell's layout (the rail's state kept in `platform`'s storage) and its one mode, Read. */
function useShell(platform: Platform): { layout: ShellLayout; modes: ModeRegistry } {
  const [shell] = useState(() => {
    const layout = new ShellLayout(platform.storage);
    const modes = new ModeRegistry();
    modes.register({ id: "read", label: message("command.mode.read"), icon: "reader", leave: () => layout.deselect() });
    return { layout, modes };
  });
  useEffect(() => void shell.layout.load(), [shell]);
  return shell;
}

/** The registry with the reader's commands, and the context they run in: focus mode, the modes and the palette are the shell's. */
function useReaderCommands(
  platform: Platform,
  reader: ReaderController,
  viewer: Viewer | undefined,
  shell: { readonly layout: ShellLayout; readonly modes: ModeRegistry; readonly openPalette: () => void },
): { registry: CommandRegistry; context: CommandContext } {
  const [registry] = useState(() => {
    const commands = new CommandRegistry();
    registerReaderCommands(commands);
    return commands;
  });
  const context = useMemo<CommandContext>(
    () => ({
      platform,
      reader,
      ...(viewer === undefined ? {} : { viewer }),
      // No rail tabs yet, so the sidebar command has no handler and stays out of every view.
      actions: { ...shellActions(shell.layout, shell.modes, { rail: false }), "palette.open": shell.openPalette },
    }),
    [platform, reader, viewer, shell],
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

/** The reader over `engine`, on `platform`; `debug` shows the debug menu (`#debug`). */
export function App({ engine, platform, debug }: { engine: Engine; platform: Platform; debug: boolean }): ReactNode {
  const { reader } = engine;
  const state = useReaderState(reader);
  const engineState = useStoreState(engine, (s) => s);
  const { backend, viewer, repairs } = engineState;
  const [palette, setPalette] = useState(false);
  const { layout, modes } = useShell(platform);
  const [shell] = useState(() => ({ layout, modes, openPalette: () => setPalette(true) }));
  const sidebar = useStoreState(layout, (s) => s.rail.open);
  // The repair panel belongs to one file: it starts closed for each.
  const [repairsFor, setRepairsFor] = useState<string | undefined>();
  const more = useGatedEntries<MoreId>([
    { id: "open", label: message("reader.open.choose") },
    { id: "rotate", label: message("reader.rotate"), icon: "rotate-right" },
  ]);
  const open = state.phase === "open";
  const shown = open ? viewer : undefined;
  const commands = useReaderCommands(platform, reader, shown, shell);
  const fileName = open ? state.file.name : undefined;
  useWindowTitle(fileName, engineState.title);
  const repaired = fileName !== undefined && repairs.length > 0;
  const showRepairs = () => {
    setRepairsFor(fileName);
    layout.openInspector({ id: "repairs", selection: false });
  };
  const status = statusItems(engineState, debug, repaired ? showRepairs : undefined);
  // Another file's repairs never show: the panel closes when the file changes.
  useEffect(() => {
    if (repairsFor !== fileName) layout.closeInspector();
  }, [repairsFor, fileName, layout]);
  const inspector =
    repaired && repairsFor === fileName ? (
      <Inspector layout={layout}>
        <RepairDetails
          fileName={fileName}
          repairs={repairs}
          onClose={() => layout.closeInspector()}
          onCopy={() => void platform.copyText(repairsText(repairs))}
        />
      </Inspector>
    ) : undefined;
  const debugMenu = debug ? (
    <DebugMenu backend={backend} onBackendChange={(next) => void engine.switchBackend(next)} keys={KEYS} />
  ) : undefined;
  return (
    <CommandsProvider registry={commands.registry} context={commands.context} keys={KEYS}>
      <ReaderShell
        layout={layout}
        topBar={
          <ReaderTopBar
            reader={reader}
            {...(shown === undefined ? {} : { viewer: shown })}
            keys={KEYS}
            sidebarOpen={sidebar}
            home={<span className="web-wordmark">{APP_NAME}</span>}
            debug={debugMenu}
            more={{ items: more, onAction: (id) => (id === "open" ? reader.close() : viewer?.rotate(90)) }}
          />
        }
        {...(inspector === undefined ? {} : { inspector })}
        status={
          <>
            {status}
            <FocusModeItem layout={layout} />
          </>
        }
      >
        {state.phase === "empty" ? (
          <OpenDropZone reader={reader} keys={KEYS}>
            <p className="web-privacy">{message("reader.open.privacy")}</p>
          </OpenDropZone>
        ) : null}
        {state.phase === "error" ? (
          <OpenFailure
            error={state.error}
            onTryAnother={() => reader.close()}
            onCopy={() => void platform.copyText(`${state.error.code}: ${state.error.message}`)}
          />
        ) : null}
        <ViewerHost viewer={viewer} visible={open} />
        <PasswordDialog reader={reader} />
      </ReaderShell>
      <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
    </CommandsProvider>
  );
}
