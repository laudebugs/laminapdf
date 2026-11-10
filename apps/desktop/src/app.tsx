// The desktop reader's window: the reader's chrome around the home screen. Opening a document
// arrives with the engine process and the tile transport (todos D2b, D2d); until then the opener
// says so.

import type { Platform } from "@laminapdf/ui";
import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  Notice,
  OpenDropZone,
  type Opener,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  registerReaderCommands,
  StatusBarItem,
  useStoreState,
} from "@laminapdf/ui";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import type { AppInfo } from "./app-info.ts";
import { NotYetSupportedError } from "./platform/desktop.ts";

const KEYS = /Mac/.test(navigator.platform) ? "mac" : "other";

const openLater: Opener = () => Promise.reject(new NotYetSupportedError("open", "D2d"));

/** Props of `App`. */
export interface AppProps {
  readonly platform: Platform;
  /** The shell's own report, or undefined while it loads. */
  readonly info: AppInfo | undefined;
}

export function App({ platform, info }: AppProps): ReactNode {
  const [reader] = useState(() => new ReaderController(platform, openLater));
  const phase = useStoreState(reader, (s) => s.phase);
  // Every open fails until the engine lands: keep the notice and go back to the home screen, whose
  // drop zone takes files only while the reader is empty.
  const [tried, setTried] = useState(false);
  useEffect(() => {
    if (phase !== "error") return;
    setTried(true);
    reader.close();
  }, [phase, reader]);
  // The reader's commands (open, the shortcuts); the shell's own handlers arrive with later todos.
  const [registry] = useState(() => {
    const commands = new CommandRegistry();
    registerReaderCommands(commands);
    return commands;
  });
  const context = useMemo<CommandContext>(() => ({ platform, reader }), [platform, reader]);
  return (
    <CommandsProvider registry={registry} context={context} keys={KEYS}>
      <ReaderShell
        topBar={
          <ReaderTopBar
            reader={reader}
            keys={KEYS}
            home={<span className="desktop-wordmark">Lamina PDF</span>}
            more={{ items: [], onAction: () => undefined }}
          />
        }
        notice={
          tried ? (
            <Notice title="Opening documents comes next" tone="neutral">
              This build of the desktop reader shows its window only. Opening a PDF arrives with its document engine.
            </Notice>
          ) : undefined
        }
        status={info === undefined ? undefined : <StatusBarItem>{`Lamina PDF ${info.version}`}</StatusBarItem>}
      >
        <OpenDropZone reader={reader} keys={KEYS} />
      </ReaderShell>
    </CommandsProvider>
  );
}
