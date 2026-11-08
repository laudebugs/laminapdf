// The reader: @laminapdf/ui's chrome around @laminapdf/viewer's view. The home screen opens a file
// from Choose a file, a drop anywhere in the window or a paste; the canvas then shows the document,
// or the failure page when it can't be opened. A repaired file gets the status bar's notice and
// the repair details panel. #debug adds the backend switch and the first page's timing.

import {
  DebugMenu,
  message,
  OpenDropZone,
  OpenFailure,
  PasswordDialog,
  type Platform,
  ReaderShell,
  ReaderTopBar,
  RepairDetails,
  RepairedNotice,
  StatusBarItem,
  useGatedEntries,
  useReaderState,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
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
  const [sidebar, setSidebar] = useState(false);
  // The repair panel belongs to one file: it starts closed for each.
  const [repairsFor, setRepairsFor] = useState<string | undefined>();
  const more = useGatedEntries<MoreId>([
    { id: "open", label: message("reader.open.choose") },
    { id: "rotate", label: message("reader.rotate"), icon: "rotate-right" },
  ]);
  const open = state.phase === "open";
  const fileName = open ? state.file.name : undefined;
  useWindowTitle(fileName, engineState.title);
  const repaired = fileName !== undefined && repairs.length > 0;
  const status = statusItems(engineState, debug, repaired ? () => setRepairsFor(fileName) : undefined);
  const inspector =
    repaired && repairsFor === fileName ? (
      <RepairDetails
        fileName={fileName}
        repairs={repairs}
        onClose={() => setRepairsFor(undefined)}
        onCopy={() => void platform.copyText(repairsText(repairs))}
      />
    ) : undefined;
  const debugMenu = debug ? (
    <DebugMenu backend={backend} onBackendChange={(next) => void engine.switchBackend(next)} keys={KEYS} />
  ) : undefined;
  return (
    <ReaderShell
      topBar={
        <ReaderTopBar
          reader={reader}
          {...(open && viewer !== undefined ? { viewer } : {})}
          keys={KEYS}
          sidebar={{ isOpen: sidebar, onChange: setSidebar }}
          home={<span className="web-wordmark">{APP_NAME}</span>}
          debug={debugMenu}
          more={{ items: more, onAction: (id) => (id === "open" ? reader.close() : viewer?.rotate(90)) }}
        />
      }
      {...(inspector === undefined ? {} : { inspector })}
      {...(status.length > 0 ? { status } : {})}
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
  );
}
