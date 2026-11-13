// Opening documents in the desktop reader: the open dialog runs in the main process, which keeps
// the path and answers with the document's id, name and pages (`open_dialog`); so do the OS's
// requests (the command line, a second launch, Finder, a drop), which arrive as events. The reader's
// controller gets a PlatformFile standing for that document, and its opener (below) shows it, or
// unlocks it first with the password the user typed (`unlock_document`). The password goes to
// the main process once and isn't kept here.

import type { Opener, PlatformFile } from "@laminapdf/ui";
import type { Viewer, ViewPosition } from "@laminapdf/viewer";
import {
  type DesktopDocument,
  type DesktopEngineClient,
  DesktopEngineError,
  type DesktopShell,
  engineError,
} from "./desktop-engine-client.ts";

/** The documents the main process opened, by the PlatformFile that stands for each in the reader. */
const documents = new WeakMap<PlatformFile, DesktopDocument>();

/** Why the main process couldn't open the file a PlatformFile stands for. */
const failures = new WeakMap<PlatformFile, DesktopEngineError>();

/** The PlatformFile that stands for `opened`: its name and size; its bytes stay in the main process. */
export function desktopFile(opened: DesktopDocument): PlatformFile {
  const file: PlatformFile = {
    name: opened.name,
    size: opened.size,
    writable: false,
    read: () => Promise.reject(new DesktopEngineError("unsupported", "the document process reads the file")),
  };
  documents.set(file, opened);
  return file;
}

/** The document `file` stands for, when the main process opened it. */
export function documentOf(file: PlatformFile): DesktopDocument | undefined {
  return documents.get(file);
}

/** Makes `file` stand for `opened` from now on: its file changed on disk and reopened, or was located again. */
export function rebind(file: PlatformFile, opened: DesktopDocument): void {
  documents.set(file, opened);
}

/**
 * Lets the user pick a PDF in the main process's dialog; empty when they cancel. A file the main
 * process couldn't open (unreadable, not a PDF) still comes back, standing for its failure, so
 * the reader's opener shows it on its failure page rather than losing it.
 */
export async function pickDocument(shell: DesktopShell): Promise<PlatformFile[]> {
  try {
    const opened = await shell.invoke<DesktopDocument | null>("open_dialog");
    return opened === null ? [] : [desktopFile(opened)];
  } catch (error) {
    const failure = engineError(error);
    // The file is open already: the main process showed its tab, so nothing opens here.
    if (failure.code === "already-open") return [];
    return [failedFile("", failure)];
  }
}

/** A PlatformFile that stands for `failure`, so the reader's opener shows it on its failure page. */
export function failedFile(name: string, failure: DesktopEngineError): PlatformFile {
  const file: PlatformFile = { name, size: 0, writable: false, read: () => Promise.reject(failure) };
  failures.set(file, failure);
  return file;
}

/** The main process's open events, sent to this window only; the same names as src-tauri/src/open.rs's. */
export const OPEN_EVENT = "open://document";
export const DRAG_ENTER_EVENT = "open://drag-enter";
export const DRAG_LEAVE_EVENT = "open://drag-leave";
export const REFUSED_EVENT = "open://refused";

/** What `OPEN_EVENT` carries: the document, or the file's name and why it didn't open. */
type OsOpened =
  | { readonly name: string; readonly document: DesktopDocument }
  | { readonly name: string; readonly error: { readonly kind: string; readonly message: string } };

/** What a drag over the window holds: how many files are PDFs and how many aren't. */
export interface Dragging {
  readonly pdfs: number;
  readonly others: number;
}

/** What the window does with the OS's open requests. */
export interface OsOpenHandlers {
  /** Shows `file` in place of what the window shows; resolves once it has opened or failed. */
  open(file: PlatformFile): Promise<void>;
  /** The file `name` couldn't be read: missing, moved or not allowed. */
  unreadable(name: string): void;
  /** Files are dragged over the window, or no longer (`undefined`). */
  dragging(drag: Dragging | undefined): void;
  /** Dropped files that aren't PDFs, by name; none of them opens. */
  refused(names: readonly string[]): void;
}

/**
 * Opens what the OS asks this window to open: files named on the command line, a second launch's,
 * Finder's and the Dock's, and dropped ones. The main process opens each and sends its id; the
 * path stays there. The listeners come first, then `take_pending_opens` hands over the requests
 * that came before the window was ready. Requests open one at a time, in order, so a quick
 * second one can't close the first while it is still opening. Returns the unsubscribe.
 */
export async function listenForOsOpens(shell: DesktopShell, handlers: OsOpenHandlers): Promise<() => void> {
  let turn = Promise.resolve();
  const show = (opened: OsOpened) => {
    if ("error" in opened && opened.error.kind === "io") return handlers.unreadable(opened.name);
    const file = "error" in opened ? failedFile(opened.name, engineError(opened.error)) : desktopFile(opened.document);
    turn = turn.then(() => handlers.open(file)).catch((error: unknown) => console.warn("opening failed", error));
  };
  const off = await Promise.all([
    shell.listen<OsOpened>(OPEN_EVENT, show),
    shell.listen<Dragging>(DRAG_ENTER_EVENT, (drag) => handlers.dragging(drag)),
    shell.listen<null>(DRAG_LEAVE_EVENT, () => handlers.dragging(undefined)),
    shell.listen<readonly string[]>(REFUSED_EVENT, (names) => handlers.refused(names)),
  ]);
  const unsubscribe = () => {
    for (const stop of off) stop();
  };
  try {
    await shell.invoke("take_pending_opens");
  } catch (error) {
    unsubscribe();
    throw error;
  }
  return unsubscribe;
}

/** What the opener tells the window's tabs (`tabs/window-tabs.ts`). */
export interface OpenerTabs {
  /** `file` is about to show: its tab becomes the shown one. Returns the view to put back. */
  showing(file: PlatformFile): ViewPosition | undefined;
  /** The reader lets go of the shown document (before it shows another): its tab keeps the view. */
  setAside(): void;
}

/**
 * The reader's opener: shows the document `file` stands for in `viewer`, unlocking it first with
 * `password`. A locked document without one fails with `password-required` and a refused one with
 * `wrong-password`, which the reader's controller turns into its password prompt. With `tabs`, the
 * shown document becomes a tab, comes back at the view it had, and is set aside rather than
 * closed when another one shows: it closes with its tab.
 */
export function desktopOpener(
  shell: DesktopShell,
  client: DesktopEngineClient,
  viewer: Viewer,
  tabs?: OpenerTabs,
): Opener {
  return async (file, password) => {
    const failed = failures.get(file);
    if (failed !== undefined) throw failed;
    let opened = documentOf(file);
    if (opened === undefined) {
      // A pasted file: the webview has its bytes but no path, and only the main process opens files.
      throw new DesktopEngineError("not-yet-supported", "open documents with the Open button or by dropping them");
    }
    if (opened.needsPassword) {
      if (password === undefined) throw new DesktopEngineError("password-required", "this document needs its password");
      let unlocked: DesktopDocument;
      try {
        unlocked = await shell.invoke<DesktopDocument>("unlock_document", { doc: opened.doc, password });
      } catch (error) {
        throw engineError(error);
      }
      opened = unlocked;
      documents.set(file, opened);
    }
    const view = tabs?.showing(file);
    viewer.open(client.show(opened));
    if (view !== undefined) viewer.restore(view);
    return {
      pageCount: opened.pageCount,
      close: () => {
        if (tabs !== undefined) {
          tabs.setAside();
          client.forget();
          return;
        }
        void client.close().catch((error: unknown) => console.warn("closing the document failed", error));
      },
    };
  };
}
