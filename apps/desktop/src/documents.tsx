// Organise's dialogs across files in the desktop window (Paper O07): Insert pages from a file,
// Replace pages and Merge documents into the open one. A file is picked in the main process's dialog and opened in this window's
// engine as a document of its own, so its pages draw as any document's do and its path never
// reaches the webview (ADR 0008). Applying gives the shown document's edit session the file as a
// source and makes the edit one step; the file then stays open as long as the document does, since
// a restarted engine needs it again to make the step again (engine/desktop-sources.ts).

import { type ExternalStore, type ToastOptions, type ToastQueue, useStoreState } from "@laminapdf/ui";
import {
  ExistsDialog,
  type FilesExist,
  type FromFileChoice,
  fromFileEdit,
  laterCountMessage,
  type MergeChoice,
  MergeDialog,
  type MergeFile,
  mergeEdit,
  type NamedPart,
  type OrganiseController,
  PagesFromFileDialog,
  type SourceFile,
  SplitDialog,
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

/** A file in Merge's list: what the person picked, and its document once it opened. */
interface MergePicked {
  /** Its key in the list, stable while it is there. */
  readonly key: string;
  readonly name: string;
  /** Undefined when it didn't open. */
  readonly doc: DesktopDocument | undefined;
  readonly shown: SourceFile;
}

/** What the dialogs across files show. */
export interface DocumentsState {
  readonly dialog: "from-file" | "replace" | "merge" | "split" | "extract" | undefined;
  /** Insert's and Replace's file. */
  readonly source: Picked | undefined;
  /** Merge's other files, in order. */
  readonly merge: readonly MergePicked[];
  /** Whether the open document is signed or certified, read as Split opens: its parts aren't. */
  readonly signed: boolean;
  /** How far Split's writing is. */
  readonly progress: { readonly done: number; readonly total: number } | undefined;
  /** The question when files of the parts' names are in the chosen folder. */
  readonly exists: FilesExist | undefined;
}

let nextKey = 1;

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
  readonly #toasts: ToastQueue;
  #state: DocumentsState = {
    dialog: undefined,
    source: undefined,
    merge: [],
    signed: false,
    progress: undefined,
    exists: undefined,
  };
  readonly #listeners = new Set<() => void>();

  constructor(client: DesktopEngineClient, organise: OrganiseController, toasts: ToastQueue) {
    this.#client = client;
    this.#organise = organise;
    this.#toasts = toasts;
  }

  /** The open file's name, which the merge's dialog names it by. */
  get fileName(): string {
    return this.#client.current?.name ?? "";
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

  /** Opens Merge documents, with the open document first and the files the person adds after it. */
  openMerge(): void {
    this.#set({ dialog: "merge" });
  }

  /** Asks for files to add to the merge, at the end, in the native dialog. */
  async addMergeFiles(): Promise<void> {
    // Another dialog is open over the grid: its own button asks for files.
    if (this.#state.dialog !== undefined && this.#state.dialog !== "merge") return;
    this.#set({ dialog: "merge" });
    let picked: Awaited<ReturnType<DesktopEngineClient["sources"]["pickMany"]>>;
    try {
      picked = await this.#client.sources.pickMany();
    } catch {
      this.#organise.say(word("documents.failed.merge"));
      return;
    }
    const files = picked.map((file): MergePicked => {
      const doc = file.opened ?? undefined;
      const shown: SourceFile =
        doc === undefined
          ? { name: file.name, size: 0, status: "unreadable", error: word("documents.file.unreadable.body") }
          : shownOf(doc, false);
      return { key: `f${nextKey++}`, name: file.name, doc, shown };
    });
    // The dialog closed while the files opened: nothing shows them.
    if (this.#state.dialog !== "merge") {
      await Promise.all(files.map((file) => this.#releaseMerge(file)));
      return;
    }
    this.#set({ merge: [...this.#state.merge, ...files] });
  }

  /** Moves merge file `key` up (-1) or down (1). */
  moveMergeFile(key: string, delta: -1 | 1): void {
    const list = [...this.#state.merge];
    const at = list.findIndex((file) => file.key === key);
    const to = at + delta;
    if (at < 0 || to < 0 || to >= list.length) return;
    [list[at], list[to]] = [list[to] as MergePicked, list[at] as MergePicked];
    this.#set({ merge: list });
  }

  /** Takes merge file `key` out of the list. */
  async removeMergeFile(key: string): Promise<void> {
    const gone = this.#state.merge.find((file) => file.key === key);
    this.#set({ merge: this.#state.merge.filter((file) => file.key !== key) });
    if (gone !== undefined) await this.#releaseMerge(gone);
  }

  /** Tries `password` on merge file `key`. */
  async mergePassword(key: string, password: string): Promise<void> {
    const file = this.#state.merge.find((f) => f.key === key);
    if (file?.doc === undefined) return;
    let next: MergePicked;
    try {
      const doc = await this.#client.sources.unlock(file.doc.doc, password);
      next = { ...file, doc, shown: shownOf(doc, false) };
    } catch (error) {
      if (!hasCode(error, "wrong-password")) {
        this.#organise.say(sourceUnreadable(file.name));
        return;
      }
      next = { ...file, shown: shownOf(file.doc, true) };
    }
    if (this.#state.merge.some((f) => f.key === key))
      this.#set({ merge: this.#state.merge.map((f) => (f.key === key ? next : f)) });
  }

  /**
   * Merges: into the open document after its last page as one step, or into a new file the person
   * chooses, which the open document stays apart from.
   */
  async merge(choice: MergeChoice): Promise<void> {
    const target = this.#client.current?.doc;
    const files = this.#state.merge.filter((file) => file.doc !== undefined && !file.doc.needsPassword);
    if (target === undefined || files.length === 0) return;
    // The dialog closes as this begins: the files not yet given to the document stay open.
    const reserved = this.#client.sources.reserve(
      files.flatMap((file) => (file.doc === undefined ? [] : [file.doc.doc])),
    );
    try {
      const titled = await this.#titled(files);
      if (choice.result === "new") await this.#mergeIntoNewFile(target, titled, choice);
      else await this.#mergeHere(target, titled, choice);
    } catch {
      this.#organise.say(word("documents.failed.merge"));
    } finally {
      reserved();
      // A file no edit took pages from closes with the dialog.
      await Promise.all(files.map((file) => this.#releaseMerge(file)));
    }
  }

  /** Each file's document and the title of its bookmark: its own, else its name without `.pdf`. */
  async #titled(files: readonly MergePicked[]): Promise<{ doc: DesktopDocument; title: string }[]> {
    return await Promise.all(
      files.map(async (file) => {
        const doc = file.doc as DesktopDocument;
        return { doc, title: (await this.#client.sources.title(doc.doc)) ?? file.name.replace(/\.pdf$/i, "") };
      }),
    );
  }

  /** Adds `files` to document `target` after its last page, as one step. */
  async #mergeHere(
    target: number,
    files: readonly { doc: DesktopDocument; title: string }[],
    choice: MergeChoice,
  ): Promise<void> {
    const sources: { source: string; title: string }[] = [];
    for (const { doc, title } of files) {
      const added = await this.#client.sources.add(target, doc.doc);
      sources.push({ source: added.id, title });
    }
    const edit = mergeEdit(choice, sources);
    this.#organise.edit(edit.ops, [], edit.label, edit.done);
  }

  /** Writes document `target` and `files` as a new file the person chooses. */
  async #mergeIntoNewFile(
    target: number,
    files: readonly { doc: DesktopDocument; title: string }[],
    choice: MergeChoice,
  ): Promise<void> {
    const stem = this.fileName.replace(/\.pdf$/i, "");
    const written = await this.#client.sources.mergeIntoNewFile(
      target,
      stem,
      word("documents.merged.name", { name: stem }),
      files.map(({ doc, title }) => ({ source: doc.doc, title })),
      choice,
    );
    if (written !== undefined) this.#organise.say(word("documents.done.saved.one", { name: written }));
  }

  /** Opens Split document, or Extract pages (Split's page ranges, from the selection). */
  openSplit(dialog: "split" | "extract"): void {
    this.#set({ dialog, progress: undefined });
    void this.#signedFacts();
  }

  /** Where Split's files go: a folder the person chooses, or the one file's own dialog. */
  output(parts: number): "folder" | "file" {
    return parts === 1 ? "file" : "folder";
  }

  /**
   * Writes `parts` as new files, which the open document stays apart from: one part to a file the
   * person chooses, several into a folder they choose, asked about first when files of those names
   * are in it. The main process writes them from the engine's parts, so no bytes reach the page.
   */
  async split(parts: readonly NamedPart[]): Promise<void> {
    const target = this.#client.current?.doc;
    if (target === undefined || parts.length === 0) return;
    const ids = this.#organise.pages().map((p) => p.id);
    const files = parts.map((part) => ({ name: part.name, pages: part.pages.map((i) => ids[i] ?? "") }));
    try {
      const done =
        files.length === 1
          ? await this.#writeOne(target, files[0] as (typeof files)[number])
          : await this.#writeMany(target, files);
      if (done) this.#set({ dialog: undefined, progress: undefined });
    } catch {
      this.#set({ progress: undefined });
      this.#organise.say(word("documents.failed.split"));
    }
  }

  /** One part to a file the person chooses; false when they cancel. */
  async #writeOne(target: number, part: { name: string; pages: string[] }): Promise<boolean> {
    const name = await this.#client.folders.savePart(target, `${part.name}.pdf`, part.pages);
    if (name === undefined) return false;
    this.#toasts.add({ message: word("documents.done.saved.one", { name }) });
    return true;
  }

  /** Several parts into a folder the person chooses, replacing or keeping both files of the same names; false when they cancel. */
  async #writeMany(target: number, files: { name: string; pages: string[] }[]): Promise<boolean> {
    const { folders } = this.#client;
    const folder = await folders.choose();
    if (folder === undefined) return false;
    const taken = await folders.existing(
      folder.id,
      files.map((file) => file.name),
    );
    const replace = taken.length === 0 ? true : await this.#ask(taken.length);
    if (replace === undefined) return false;
    // One part at a time, so the dialog shows how far it is. Only a file there before the split is
    // replaced: a part never replaces another part of its own split.
    const written: string[] = [];
    this.#set({ progress: { done: 0, total: files.length } });
    for (const file of files) {
      const part = { ...file, replace: replace && taken.includes(file.name) };
      written.push(...(await folders.writeParts(target, folder.id, [part])));
      this.#set({ progress: { done: written.length, total: files.length } });
    }
    this.#toasts.add(this.#writtenToast(folder, written));
    return true;
  }

  /** The toast after a split into a folder: how many files, and an offer to show the first. */
  #writtenToast(folder: { id: number; name: string }, written: readonly string[]): ToastOptions {
    const first = written[0];
    return {
      message: laterCountMessage(
        written.length,
        { one: "documents.done.folder.one", many: "documents.done.folder" },
        { folder: folder.name },
      ),
      ...(first === undefined
        ? {}
        : {
            action: {
              label: word("documents.done.reveal"),
              onPress: () => void this.#client.folders.reveal(folder.id, first).catch(() => undefined),
            },
          }),
    };
  }

  /** Asks whether to replace the `count` files of the parts' names: true, keep both (false), or cancel (undefined). */
  #ask(count: number): Promise<boolean | undefined> {
    return new Promise((resolve) =>
      this.#set({
        exists: {
          count,
          answer: (replace) => {
            this.#set({ exists: undefined });
            resolve(replace);
          },
        },
      }),
    );
  }

  /** Whether the open document is signed, read as Split opens. */
  async #signedFacts(): Promise<void> {
    const plan = await this.#client.planSave({}).catch(() => undefined);
    this.#set({ signed: (plan?.signatures.length ?? 0) > 0 });
  }

  /** Lets go of merge file `file`: it closes unless an edit took pages from it. */
  async #releaseMerge(file: MergePicked): Promise<void> {
    if (file.doc !== undefined) await this.#client.sources.release(file.doc.doc);
  }

  /** Closes the dialog; the files close with it unless an edit took pages from them. */
  async close(): Promise<void> {
    // Writing the parts can't be stopped half way.
    if (this.#state.progress !== undefined) return;
    // A question about files that exist, still open, is answered no.
    this.#state.exists?.answer(undefined);
    const { source, merge } = this.#state;
    this.#set({ dialog: undefined, source: undefined, merge: [] });
    if (source !== undefined) await this.#client.sources.release(source.doc.doc);
    await Promise.all(merge.map((file) => this.#releaseMerge(file)));
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
    "pages.extract": () => documents.openSplit("extract"),
    "document.split": () => documents.openSplit("split"),
    "document.merge": () => documents.openMerge(),
    "document.merge-add-files": () => void documents.addMergeFiles(),
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
  const name = documents.fileName.replace(/\.pdf$/i, "");
  const mergeFiles: MergeFile[] = state.merge.map((file) => ({ key: file.key, source: file.shown }));
  return (
    <>
      <PagesFromFileDialog
        isOpen={state.dialog === "from-file" || state.dialog === "replace"}
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
      <SplitDialog
        isOpen={state.dialog === "split" || state.dialog === "extract"}
        onClose={close}
        start={state.dialog === "extract" ? "ranges" : "count"}
        labels={labels}
        selected={selected}
        chapters={[]}
        stem={name}
        output={(parts) => documents.output(parts)}
        signed={state.signed}
        progress={state.progress}
        onSplit={(parts) => void documents.split(parts)}
      />
      <ExistsDialog ask={state.exists} />
      <MergeDialog
        isOpen={state.dialog === "merge"}
        onClose={close}
        current={{ name, claims: [] }}
        files={mergeFiles}
        onAdd={() => void documents.addMergeFiles()}
        onMove={(key, delta) => documents.moveMergeFile(key, delta)}
        onRemove={(key) => void documents.removeMergeFile(key)}
        onPassword={(key, password) => void documents.mergePassword(key, password)}
        onMerge={(choice) => void documents.merge(choice)}
      />
    </>
  );
}
