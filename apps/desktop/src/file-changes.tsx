// When the open document's file changes on disk (S04): a settled change shows the new version at
// the same page and zoom, with a toast; a file moved or deleted shows a strip under the top bar,
// with Locate… (choose where it went: the view moves to that file) and Close, while the document
// stays readable; a change the main process couldn't open shows the unreadable notice until the next good
// reload. Each event names a document id, and only the one on screen counts. A tab of the last
// session whose file is gone shows the same words in place of the document, with Locate… and Close.

import { message, Notice, type PlatformFile, type ReaderController, type ToastQueue } from "@laminapdf/ui";
import { type ReactNode, useEffect, useState } from "react";
import type { DesktopEngine } from "./app.tsx";
import { type DesktopDocument, engineError } from "./engine/desktop-engine-client.ts";
import { listenForFileChanges, showInPlace } from "./engine/file-changes.ts";
import { desktopFile } from "./engine/open.ts";
import { stopper } from "./engine/stopper.ts";
import type { WindowTabs } from "./tabs/window-tabs.ts";

/** What is wrong with the open document's file, if anything. */
type FileProblem = "missing" | "unreadable";

/**
 * Follows the open document's file and returns the strip to show under the top bar, if any.
 * @param engine - This window's engine; undefined outside the shell, where nothing is watched.
 */
export function useFileChanges(
  engine: DesktopEngine | undefined,
  reader: ReaderController,
  toasts: ToastQueue,
  tabs?: WindowTabs,
): ReactNode {
  const [problem, setProblem] = useState<{ doc: number; kind: FileProblem }>();
  useEffect(() => {
    if (engine === undefined) return;
    const shown = () => engine.client.current?.doc;
    return stopper(
      listenForFileChanges(engine.shell, {
        reloaded: (previous, document) => {
          if (shown() !== previous) {
            // A tab not shown: it takes the new document quietly, and shows it when next shown.
            if (tabs?.reloaded(previous, document) === true) return;
            // The window moved on (closed it, opened another) while it reloaded: nothing shows the
            // new document, so it closes, and its file stops being watched for this window.
            void engine.shell.invoke("close_document", { doc: document.doc }).catch(() => undefined);
            return;
          }
          show(engine, reader, document);
          setProblem(undefined);
          toasts.add({ message: message("reader.file.reloaded", { name: document.name }) });
        },
        missing: (doc) => {
          if (shown() === doc) setProblem({ doc, kind: "missing" });
        },
        unreadable: (doc) => {
          if (shown() === doc) setProblem({ doc, kind: "unreadable" });
        },
      }),
      "listening for file changes",
    );
  }, [engine, reader, toasts, tabs]);
  // Another document on screen: the strip was about the old one.
  const doc = engine?.client.current?.doc;
  if (problem === undefined || engine === undefined || problem.doc !== doc) return null;
  if (problem.kind === "unreadable") return <Notice title={message("reader.file.unreadable")} tone="warning" />;
  return (
    <Notice title={message("reader.file.missing.title")} tone="warning">
      {message("reader.file.missing.body")}{" "}
      <button
        type="button"
        className="desktop-link"
        onClick={() =>
          void locate(engine, reader).then(
            (located) => located && setProblem(undefined),
            (error: unknown) => toasts.add({ message: engineError(error).message }),
          )
        }
      >
        {message("reader.file.missing.locate")}
      </button>{" "}
      <button type="button" className="desktop-link" onClick={() => reader.close()}>
        {message("reader.file.missing.close")}
      </button>
    </Notice>
  );
}

/**
 * Shows `document`, the reader's document reopened from its changed file, where the reader was.
 * One that now needs a password goes through the reader's open, which asks for it.
 */
function show(engine: DesktopEngine, reader: ReaderController, document: DesktopDocument): void {
  const state = reader.getState();
  if (state.phase !== "open" || document.needsPassword) {
    void reader.open(desktopFile(document));
    return;
  }
  showInPlace(engine.client, engine.viewer, state.file, document);
}

/**
 * Lets the user choose where the missing file went, and shows it in place of the one on screen,
 * which then closes. Resolves to whether a file was chosen.
 */
async function locate(engine: DesktopEngine, reader: ReaderController): Promise<boolean> {
  const located = await engine.shell.invoke<DesktopDocument | null>("open_dialog");
  if (located === null) return false;
  const previous = engine.client.current?.doc;
  show(engine, reader, located);
  if (previous !== undefined && !located.needsPassword) await engine.shell.invoke("close_document", { doc: previous });
  return true;
}

/**
 * The page a tab of the last session shows when its file couldn't be found (D4d): Locate… lets
 * the user choose where it went, and the tab shows that file; Close closes the tab.
 */
export function MissingRestoredTab({
  engine,
  tabs,
  file,
  toasts,
}: {
  engine: DesktopEngine;
  tabs: WindowTabs;
  file: PlatformFile;
  toasts: ToastQueue;
}): ReactNode {
  const id = tabs.getState().tabs.find((tab) => tab.file === file)?.id;
  if (id === undefined) return null;
  const locate = async () => {
    const located = await engine.shell.invoke<DesktopDocument | null>("open_dialog");
    if (located !== null) await tabs.located(id, located);
  };
  return (
    <div className="desktop-missing">
      <Notice title={message("reader.file.missing.title")} tone="warning">
        {message("reader.file.missing.restored")}{" "}
        <button
          type="button"
          className="desktop-link"
          onClick={() => void locate().catch((error: unknown) => toasts.add({ message: engineError(error).message }))}
        >
          {message("reader.file.missing.locate")}
        </button>{" "}
        <button type="button" className="desktop-link" onClick={() => void tabs.close(id)}>
          {message("reader.file.missing.close")}
        </button>
      </Notice>
    </div>
  );
}
