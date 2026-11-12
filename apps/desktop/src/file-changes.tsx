// When the open document's file changes on disk (S04): a settled change shows the new version at
// the same page and zoom, with a toast; a file moved or deleted shows a strip under the top bar,
// with Locate… (choose where it went: the view moves to that file) and Close, while the document
// stays readable; a change the main process couldn't open shows "retrying" until the next good
// reload. Each event names a document id, and only the one on screen counts.

import { message, Notice, type ReaderController, type ToastQueue } from "@laminapdf/ui";
import { type ReactNode, useEffect, useState } from "react";
import type { DesktopEngine } from "./app.tsx";
import { type DesktopDocument, engineError } from "./engine/desktop-engine-client.ts";
import { listenForFileChanges, showInPlace } from "./engine/file-changes.ts";
import { desktopFile } from "./engine/open.ts";
import { stopper } from "./engine/stopper.ts";

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
): ReactNode {
  const [problem, setProblem] = useState<{ doc: number; kind: FileProblem }>();
  useEffect(() => {
    if (engine === undefined) return;
    const shown = () => engine.client.current?.doc;
    return stopper(
      listenForFileChanges(engine.shell, {
        reloaded: (previous, document) => {
          if (shown() !== previous) {
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
  }, [engine, reader, toasts]);
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
