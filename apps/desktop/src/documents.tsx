// Organise's dialogs across files in the desktop window (Paper O07): Insert pages from a file and
// Replace pages. The file is picked in the main process's dialog and opened in this window's
// engine as a document of its own, so its pages draw as any document's do and its path never
// reaches the webview (ADR 0008). Applying gives the shown document's edit session the file as a
// source and makes the edit one step; the file then stays open as long as the document does, since
// a restarted engine needs it again to make the step again (engine/desktop-sources.ts).

import { type ExternalStore, useStoreState } from "@laminapdf/ui";
import {
  type FromFileChoice,
  fromFileEdit,
  type OrganiseController,
  PagesFromFileDialog,
  type SourceFile,
  sourceUnreadable,
  laterMessage as word,
} from "@laminapdf/ui/editing";
import { type ReactNode, useCallback, useEffect } from "react";
import type { DesktopDocument, DesktopEngineClient } from "./engine/desktop-engine-client.ts";
import type { DesktopOrganiseEngine } from "./organise.tsx";

/** A file a dialog reads pages from: the document the main process opened for it, as the dialog shows it. */
interface Picked {
  readonly doc: DesktopDocument;
  readonly shown: SourceFile;
}

/** What the dialogs across files show. */
export interface DocumentsState {
  readonly dialog: "from-file" | "replace" | undefined;
  readonly source: Picked | undefined;
}

/** Whether `error` is the main process's error `code`. */
function hasCode(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && (error as { code?: unknown }).code === code;
}

/** `doc` as the dialog shows it: ready, or asking for its password. */
function shownOf(doc: DesktopDocument, wrong: boolean): SourceFile {
  const { name, size } = doc;
  if (doc.needsPassword) return { name, size, status: "password", wrong };
  return { name, size, status: "ready", pages: doc.pages.map(({ width, height }) => ({ width, height })) };
}

/** The dialogs across files over the shown document, and the file they read. */
export class DesktopDocumentsController implements ExternalStore<DocumentsState> {
  readonly #client: DesktopEngineClient;
  readonly #organise: OrganiseController;
  #state: DocumentsState = { dialog: undefined, source: undefined };
  readonly #listeners = new Set<() => void>();

  constructor(client: DesktopEngineClient, organise: OrganiseController) {
    this.#client = client;
    this.#organise = organise;
  }

  getState(): DocumentsState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Opens Insert pages from a file or Replace pages, asking for the file first. */
  async openFromFile(mode: "from-file" | "replace"): Promise<void> {
    this.#set({ dialog: mode });
    if (this.#state.source === undefined) await this.chooseSource();
  }

  /** Asks for the file Insert or Replace reads, in the native dialog. */
  async chooseSource(): Promise<void> {
    let doc: DesktopDocument | undefined;
    try {
      doc = await this.#client.sources.pick();
    } catch {
      // Not a PDF, or damaged beyond repair: the dialog stays as it was.
      this.#organise.say(word("documents.file.unreadable.body"));
      return;
    }
    if (doc === undefined) return;
    const old = this.#state.source;
    // The dialog closed while the file opened: nothing shows it.
    if (this.#state.dialog === undefined) return void (await this.#client.sources.release(doc.doc));
    this.#set({ source: { doc, shown: shownOf(doc, false) } });
    if (old !== undefined) await this.#client.sources.release(old.doc.doc);
  }

  /** Tries `password` on Insert's or Replace's encrypted file. */
  async sourcePassword(password: string): Promise<void> {
    const source = this.#state.source;
    if (source === undefined) return;
    try {
      const doc = await this.#client.sources.unlock(source.doc.doc, password);
      if (this.#state.source?.doc.doc === doc.doc) this.#set({ source: { doc, shown: shownOf(doc, false) } });
    } catch (error) {
      if (!hasCode(error, "wrong-password")) {
        this.#organise.say(sourceUnreadable(source.doc.name));
        return;
      }
      if (this.#state.source?.doc.doc === source.doc.doc)
        this.#set({ source: { ...source, shown: shownOf(source.doc, true) } });
    }
  }

  /** Draws page `page` of `picked` into `canvas`, at the canvas box's size in device pixels. */
  async draw(picked: Picked | undefined, page: number, canvas: HTMLCanvasElement, signal: AbortSignal): Promise<void> {
    const size = picked?.doc.pages[page];
    const box = canvas.parentElement;
    if (picked === undefined || size === undefined || box === null) return;
    const ratio = globalThis.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(box.clientWidth * ratio));
    const { bitmap } = await this.#client.renderSource(picked.doc, { page, scale: width / size.width }, { signal });
    try {
      if (signal.aborted) return;
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
    } finally {
      bitmap.close();
    }
  }

  /** Brings the chosen pages of Insert's or Replace's file in, as one step. */
  async applyFromFile(choice: FromFileChoice): Promise<void> {
    const source = this.#state.source;
    const target = this.#client.current?.doc;
    if (source === undefined || target === undefined) return;
    const pages = this.#organise.pages().map((p) => p.id);
    let added: Awaited<ReturnType<DesktopEngineClient["sources"]["add"]>>;
    try {
      added = await this.#client.sources.add(target, source.doc.doc);
    } catch {
      this.#organise.say(sourceUnreadable(source.doc.name));
      // The dialog closed as this began, and left the file for the add to keep.
      await this.#client.sources.release(source.doc.doc);
      return;
    }
    const edit = fromFileEdit(choice, added, pages, this.#organise.selectedIndices());
    this.#organise.edit(edit.ops, edit.indices, edit.label, edit.done);
  }

  /** Closes the dialog; the file closes with it unless an edit took pages from it. */
  async close(): Promise<void> {
    const source = this.#state.source;
    this.#set({ dialog: undefined, source: undefined });
    if (source !== undefined) await this.#client.sources.release(source.doc.doc);
  }

  #set(change: Partial<DocumentsState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** The handlers of the commands across files while Organise is the mode. */
export function documentsActions(documents: DesktopDocumentsController) {
  return {
    "pages.insert-from-file": () => void documents.openFromFile("from-file"),
    "pages.replace": () => void documents.openFromFile("replace"),
  } as const;
}

/** The dialogs across files. */
export function DesktopDocumentsDialogs({
  documents,
  organise,
  pages: engine,
}: {
  readonly documents: DesktopDocumentsController;
  readonly organise: OrganiseController;
  readonly pages: DesktopOrganiseEngine;
}): ReactNode {
  const state = useStoreState(documents, (s) => s);
  const pages = useStoreState(engine, (s) => s.pages);
  const selectedIds = useStoreState(organise, (s) => s.selected);
  const selected = pages.flatMap((p, i) => (selectedIds.has(p.id) ? [i] : []));
  const labels = pages.map((p, i) => p.label ?? String(i + 1));
  const close = () => void documents.close();
  // The dialogs go with the document: the file they read closes with them.
  useEffect(() => () => void documents.close(), [documents]);
  const source = state.source;
  const draw = useCallback(
    (page: number, canvas: HTMLCanvasElement, signal: AbortSignal) => documents.draw(source, page, canvas, signal),
    [documents, source],
  );
  return (
    <PagesFromFileDialog
      isOpen={state.dialog !== undefined}
      onClose={close}
      mode={state.dialog === "replace" ? "replace" : "insert"}
      source={source?.shown}
      onChoose={() => void documents.chooseSource()}
      onPassword={(password) => void documents.sourcePassword(password)}
      draw={draw}
      labels={labels}
      selected={selected}
      onApply={(choice) => void documents.applyFromFile(choice)}
    />
  );
}
