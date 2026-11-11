// Opening documents in the desktop reader: the open dialog runs in the main process, which keeps
// the path and answers with the document's id, name and pages (`open_dialog`). The reader's
// controller gets a PlatformFile standing for that document, and its opener (below) shows it, or
// unlocks it first with the password the user typed (`unlock_document`). The password goes to
// the main process once and isn't kept here.

import type { Opener, PlatformFile } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
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
function documentOf(file: PlatformFile): DesktopDocument | undefined {
  return documents.get(file);
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
    const file: PlatformFile = { name: "", size: 0, writable: false, read: () => Promise.reject(failure) };
    failures.set(file, failure);
    return [file];
  }
}

/**
 * The reader's opener: shows the document `file` stands for in `viewer`, unlocking it first with
 * `password`. A locked document without one fails with `password-required` and a refused one with
 * `wrong-password`, which the reader's controller turns into its password prompt.
 */
export function desktopOpener(shell: DesktopShell, client: DesktopEngineClient, viewer: Viewer): Opener {
  return async (file, password) => {
    const failed = failures.get(file);
    if (failed !== undefined) throw failed;
    let opened = documentOf(file);
    if (opened === undefined) {
      // Dropped and pasted files come with opening from the OS: only the open dialog reaches the main process.
      throw new DesktopEngineError("not-yet-supported", "open documents with the Open button for now");
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
    viewer.open(client.show(opened));
    return {
      pageCount: opened.pageCount,
      close: () => {
        void client.close().catch((error: unknown) => console.warn("closing the document failed", error));
      },
    };
  };
}
