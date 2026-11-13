// Files that change on disk while they are open (src-tauri/src/watch/): the main process watches
// each open document's file and, once a change settles, reopens it under a new id and closes the
// old one (`document://reloaded`), or says the file is gone (`document://missing`) or changed into
// something it couldn't open (`document://unreadable`). The window shows the new document where
// the reader was: the same page, place and zoom, the page clamped when the document got shorter.

import type { PlatformFile } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import type { DesktopDocument, DesktopEngineClient, DesktopShell } from "./desktop-engine-client.ts";
import { rebind } from "./open.ts";

/** The main process's file events, sent to this window only; the same names as src-tauri/src/watch/app.rs's. */
export const RELOADED_EVENT = "document://reloaded";
export const MISSING_EVENT = "document://missing";
export const UNREADABLE_EVENT = "document://unreadable";

/** `RELOADED_EVENT`'s payload: the id the window knew the document by, and the document now. */
export interface Reloaded {
  readonly previous: number;
  readonly document: DesktopDocument;
}

/** What the window does when its document's file changes. */
export interface FileChangeHandlers {
  /** The document `previous` reopened from its changed file as `document`. */
  reloaded(previous: number, document: DesktopDocument): void;
  /** The file behind document `doc` is gone; the document stays readable. */
  missing(doc: number): void;
  /** The file behind document `doc` changed but couldn't be opened; the old view stays. */
  unreadable(doc: number): void;
}

/** Listens to this window's file events; resolves to the unsubscribe. */
export async function listenForFileChanges(shell: DesktopShell, handlers: FileChangeHandlers): Promise<() => void> {
  const off = await Promise.all([
    shell.listen<Reloaded>(RELOADED_EVENT, ({ previous, document }) => handlers.reloaded(previous, document)),
    shell.listen<{ doc: number }>(MISSING_EVENT, ({ doc }) => handlers.missing(doc)),
    shell.listen<{ doc: number }>(UNREADABLE_EVENT, ({ doc }) => handlers.unreadable(doc)),
  ]);
  return () => {
    for (const stop of off) stop();
  };
}

/**
 * Shows `document` in place of the one `file` stands for, where the reader was: the view's
 * position is kept and restored, its page clamped to the new page count. The reader's file now
 * stands for `document`, so closing it closes the new one.
 */
export function showInPlace(
  client: DesktopEngineClient,
  viewer: Viewer,
  file: PlatformFile,
  document: DesktopDocument,
): void {
  const at = viewer.viewPosition();
  rebind(file, document);
  viewer.open(client.show(document));
  if (at !== undefined) viewer.restore(at);
}
