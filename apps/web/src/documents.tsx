// Organise's dialogs across files in the reader (Paper O07, O08; E2m): Insert pages from a file,
// Replace pages, Merge documents, Split document and Extract pages. A file the person picks is
// read whole and previewed in the document worker (its pages drawn as thumbnails, its claims read);
// once they apply, it becomes a source of the open document and the edit is one step of the
// session, asked about first when its save would break a signature (OrganiseController.edit). Split
// and Extract write new files and leave the document as it is: into a folder the person picks where
// the browser can (`canWriteDirectory`), else one file, two downloads, or one zip of them all.
// Loaded with the Organise mode (later.ts).

import {
  Button,
  Dialog,
  type ExternalStore,
  type Platform,
  type PlatformFile,
  type PlatformFolder,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import { type ImagesToPdfChoice, ImagesToPdfDialog } from "@laminapdf/ui/dialogs";
import {
  type FromFileChoice,
  fromFileEdit,
  type MergeChoice,
  MergeDialog,
  type MergeFile,
  mergeEdit,
  type NamedPart,
  PagesFromFileDialog,
  type SourceFile,
  SplitDialog,
  sourceUnreadable,
  laterMessage as word,
  laterCountMessage as words,
} from "@laminapdf/ui/editing";
import {
  closeSource,
  mergeFiles,
  previewSource,
  renderSource,
  type SourcePreview,
  type ViewerClient,
  writeParts,
} from "@laminapdf/viewer";
import { type ReactNode, useCallback, useEffect } from "react";
import { downloadName, downloadStem } from "./download-name.ts";
import { freeName } from "./free-name.ts";
import type { OrganiseController } from "./organise.tsx";
import type { Engine } from "./session.ts";

/** A file a dialog reads pages from: what was picked, its bytes once read, and its preview. */
interface Picked {
  readonly key: string;
  readonly file: PlatformFile;
  readonly bytes: Uint8Array | undefined;
  /** The password that opened it, kept until the dialog closes; never sent with an op. */
  readonly password: string | undefined;
  readonly preview: SourcePreview | undefined;
  readonly shown: SourceFile;
}

/** The dialog across files that is open. */
type DocumentsDialog = "from-file" | "replace" | "merge" | "split" | "extract" | "images";

/** What the dialogs across files show. */
export interface DocumentsState {
  readonly dialog: DocumentsDialog | undefined;
  /** Insert's and Replace's file. */
  readonly source: Picked | undefined;
  /** Merge's other files, in order. */
  readonly merge: readonly Picked[];
  /** The open document's PDF/A and PDF/UA claims, read as Merge opens. */
  readonly claims: readonly string[];
  /** Split's top-level bookmarks, read as it opens. */
  readonly chapters: readonly { readonly title: string; readonly page: number }[];
  /** Whether the open document is signed or certified, read as Split opens. */
  readonly signed: boolean;
  /** How far Split's writing is. */
  readonly progress: { readonly done: number; readonly total: number } | undefined;
  /** The question when files of the parts' names are in the chosen folder: how many, and the answer's taker. */
  readonly exists: { readonly count: number; readonly answer: (replace: boolean | undefined) => void } | undefined;
  /** The pictures From images makes a PDF of, in the order chosen. */
  readonly images: readonly PlatformFile[];
}

/** The claims a preview's metadata makes, as the merge's notice names them. */
function claimsOf(pdfa: string | undefined, pdfua: number | undefined): string[] {
  return [...(pdfa === undefined ? [] : [`PDF/A-${pdfa}`]), ...(pdfua === undefined ? [] : [`PDF/UA-${pdfua}`])];
}

/** Whether `error` is Lamina PDF's code `code`. */
function hasCode(error: unknown, code: string): boolean {
  return typeof error === "object" && error !== null && (error as { code?: unknown }).code === code;
}

let nextKey = 1;

/** A folder the person chose to write into. */
type Folder = PlatformFolder;

/** Thrown when the person cancels a save dialog or the question about files already in the folder: nothing more is written. */
class Cancelled extends Error {}

/** The worker answered without the file asked for, which only a bug does. */
function noFile(): never {
  throw new Error("the document worker wrote no such file");
}

/** The dialogs across files over one open document, and the files they read. */
export class DocumentsController implements ExternalStore<DocumentsState> {
  readonly #engine: Engine;
  readonly #organise: OrganiseController;
  readonly #toasts: ToastQueue;
  readonly #platform: Platform;
  #state: DocumentsState = {
    dialog: undefined,
    source: undefined,
    merge: [],
    claims: [],
    chapters: [],
    signed: false,
    progress: undefined,
    exists: undefined,
    images: [],
  };
  readonly #listeners = new Set<() => void>();

  constructor(options: {
    readonly engine: Engine;
    readonly organise: OrganiseController;
    readonly toasts: ToastQueue;
    readonly platform: Platform;
  }) {
    this.#engine = options.engine;
    this.#organise = options.organise;
    this.#toasts = options.toasts;
    this.#platform = options.platform;
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

  /** Asks for the file Insert or Replace reads, and previews it. */
  async chooseSource(): Promise<void> {
    const [file] = await this.#platform.openFiles({ multiple: false });
    if (file === undefined) return;
    const old = this.#state.source;
    const picked = this.#picked(file);
    this.#set({ source: picked });
    await this.#close(old);
    const read = await this.#read(picked, undefined);
    // The dialog closed, or another file was chosen, while this one was read: its preview goes.
    if (this.#state.source?.key === picked.key) this.#set({ source: read });
    else await this.#close(read);
  }

  /** Tries `password` on Insert's or Replace's encrypted file. */
  async sourcePassword(password: string): Promise<void> {
    const source = this.#state.source;
    if (source === undefined) return;
    const read = await this.#read(source, password);
    if (this.#state.source?.key === source.key) this.#set({ source: read });
    else await this.#close(read);
  }

  /** Draws page `page` of `picked` into `canvas`, at the canvas box's size in device pixels. */
  async draw(picked: Picked | undefined, page: number, canvas: HTMLCanvasElement, signal: AbortSignal): Promise<void> {
    const preview = picked?.preview;
    const size = preview?.pages[page];
    const box = canvas.parentElement;
    if (preview === undefined || size === undefined || box === null) return;
    const ratio = globalThis.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(box.clientWidth * ratio));
    const { client } = await this.#engine.view();
    const { bitmap } = await renderSource(client, preview.key, page, width / size.width, { signal });
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
    if (source?.bytes === undefined) return;
    const pages = this.#organise.pages();
    const { client } = await this.#engine.view();
    let added: Awaited<ReturnType<ViewerClient["addSource"]>>;
    try {
      added = await client.addSource(source.bytes, source.password === undefined ? {} : { password: source.password });
    } catch {
      this.#organise.say(sourceUnreadable(source.file.name));
      return;
    }
    const edit = fromFileEdit(
      choice,
      added,
      pages.map((p) => p.id),
      this.#organise.selectedIndices(),
    );
    this.#organise.edit(edit.ops, edit.indices, edit.label, edit.done);
  }

  /** Opens Merge documents, with the open document first and `files` after it. */
  async openMerge(files: readonly PlatformFile[] = []): Promise<void> {
    this.#set({ dialog: "merge" });
    void this.#claims();
    await this.#addToMerge(files);
  }

  /** From images (O07): makes a new PDF of pictures, one page each; asks for them first. */
  async openImages(): Promise<void> {
    this.#set({ dialog: "images" });
    if (this.#state.images.length === 0) await this.chooseImages();
  }

  /** Asks for pictures to add to From images, at the end. */
  async chooseImages(): Promise<void> {
    const chosen = await this.#platform.openFiles({ multiple: true, accept: "images" });
    if (chosen.length > 0) this.#set({ images: [...this.#state.images, ...chosen] });
  }

  /**
   * Makes the new PDF of the pictures, in `choice.order`, and saves it as a file of its own, named
   * after the first picture: like Merge's new document, it doesn't replace the one open here.
   */
  async makeImagesPdf(choice: ImagesToPdfChoice): Promise<void> {
    const images = choice.order.flatMap((i) => this.#state.images[i] ?? []);
    const first = images[0];
    if (first === undefined) return;
    // Named after the first picture, made safe as any new file's name is.
    const name = downloadName(first.name.replace(/\.[^.]+$/, ""));
    try {
      const { client } = await this.#engine.view();
      const inputs = await Promise.all(
        images.map(async (file) => ({
          bytes: new Uint8Array(await file.read(0, file.size)),
          type: imageType(file.name),
        })),
      );
      const made = client.imagesToPdf(inputs, { pageSize: choice.pageSize, margin: choice.margin });
      await this.#saveOne(made, name);
    } catch (error) {
      if (error instanceof Cancelled) return;
      reportError(error);
      this.#organise.say(word("imagesToPdf.failed"));
      return;
    }
    this.#set({ dialog: undefined, images: [] });
    this.#toasts.add({
      message: words(1, { one: "documents.done.saved.one", many: "documents.done.saved" }, { name }),
    });
  }

  /** Asks for files to add to the merge, at the end. */
  async addMergeFiles(): Promise<void> {
    if (this.#state.dialog !== "merge") this.#set({ dialog: "merge" });
    await this.#addToMerge(await this.#platform.openFiles({ multiple: true }));
  }

  /** Moves merge file `key` up (-1) or down (1). */
  moveMergeFile(key: string, delta: -1 | 1): void {
    const list = [...this.#state.merge];
    const at = list.findIndex((p) => p.key === key);
    const to = at + delta;
    if (at < 0 || to < 0 || to >= list.length) return;
    [list[at], list[to]] = [list[to] as Picked, list[at] as Picked];
    this.#set({ merge: list });
  }

  /** Takes merge file `key` out of the list. */
  async removeMergeFile(key: string): Promise<void> {
    const gone = this.#state.merge.find((p) => p.key === key);
    this.#set({ merge: this.#state.merge.filter((p) => p.key !== key) });
    await this.#close(gone);
  }

  /** Tries `password` on merge file `key`. */
  async mergePassword(key: string, password: string): Promise<void> {
    const picked = this.#state.merge.find((p) => p.key === key);
    if (picked === undefined) return;
    await this.#replaceMergeFile(await this.#read(picked, password));
  }

  /** Puts `read` in its file's place in the merge, or closes its preview when the file left the list. */
  async #replaceMergeFile(read: Picked): Promise<void> {
    if (this.#state.merge.some((p) => p.key === read.key)) {
      this.#set({ merge: this.#state.merge.map((p) => (p.key === read.key ? read : p)) });
    } else {
      await this.#close(read);
    }
  }

  /** Merges: into the open document as one step, or into a new document, saved as a new file named from `stem`. */
  async merge(choice: MergeChoice, stem: string): Promise<void> {
    const files = this.#state.merge.filter((p) => p.bytes !== undefined);
    const { client } = await this.#engine.view();
    const options = { outline: choice.bookmarks, fields: choice.fields } as const;
    if (choice.result === "new") {
      const inputs = files.map((p) => ({
        bytes: p.bytes as Uint8Array,
        title: p.preview?.title ?? p.file.name.replace(/\.pdf$/i, ""),
        ...(p.password === undefined ? {} : { password: p.password }),
      }));
      try {
        const merged = mergeFiles(client, inputs, { open: stem, merge: options });
        await this.#saveOne(merged, word("documents.merged.name", { name: stem }));
      } catch (error) {
        if (!(error instanceof Cancelled)) this.#organise.say(word("documents.failed.merge"));
      }
      return;
    }
    const sources: { source: string; title: string }[] = [];
    try {
      for (const p of files) {
        const added = await client.addSource(
          p.bytes as Uint8Array,
          p.password === undefined ? {} : { password: p.password },
        );
        sources.push({ source: added.id, title: p.preview?.title ?? p.file.name.replace(/\.pdf$/i, "") });
      }
    } catch {
      this.#organise.say(word("documents.failed.merge"));
      return;
    }
    const edit = mergeEdit(choice, sources);
    this.#organise.edit(edit.ops, [], edit.label, edit.done);
  }

  /** Opens Split document, or Extract pages (Split's page ranges, from the selection). */
  openSplit(dialog: "split" | "extract"): void {
    this.#set({ dialog, progress: undefined });
    void this.#splitFacts();
  }

  /** Where Split's files go here: a folder the person picks, one zip, or each file on its own. */
  output(parts: number): "folder" | "zip" | "files" | "file" {
    if (parts === 1) return "file";
    if (this.#platform.capabilities.canWriteDirectory) return "folder";
    return parts > 2 ? "zip" : "files";
  }

  /**
   * Writes `parts` as new files. A folder is asked for first, while the click that started it
   * still lets the browser show its picker; the document worker then writes the files.
   */
  async split(parts: readonly NamedPart[], stem: string): Promise<void> {
    const ids = this.#organise.pages().map((p) => p.id);
    const ranges = parts.map((part) => part.pages.map((i) => ids[i] ?? ""));
    const names = parts.map((part) => downloadName(part.name));
    const how = this.output(parts.length);
    const folder = how === "folder" ? await this.#platform.chooseFolder?.() : undefined;
    if (how === "folder" && folder === undefined) return;
    this.#set({ progress: { done: 0, total: parts.length } });
    try {
      await this.#write(ranges, names, how === "zip" ? `${stem} parts.zip` : folder);
    } catch (error) {
      if (!(error instanceof Cancelled)) this.#organise.say(word("documents.failed.split"));
      this.#set({ progress: undefined });
      return;
    }
    this.#set({ dialog: undefined, progress: undefined });
    this.#toasts.add({
      message:
        folder === undefined
          ? words(
              parts.length,
              { one: "documents.done.saved.one", many: "documents.done.saved" },
              { name: names[0] ?? "" },
            )
          : words(
              parts.length,
              { one: "documents.done.folder.one", many: "documents.done.folder" },
              { folder: folder.name },
            ),
    });
  }

  /**
   * Has the document worker write one file per list of `ranges`, named `names`, then puts them
   * `where`: into a folder, as one zip saved under that name, or (undefined) each saved on its own.
   * A save dialog opens before the worker is done, while the click still lets the browser show it.
   */
  async #write(ranges: string[][], names: readonly string[], where: Folder | string | undefined): Promise<void> {
    const { client } = await this.#engine.view();
    const onProgress = (progress: { done: number; total: number }) => this.#set({ progress });
    if (typeof where === "string") {
      const zipped = writeParts(client, ranges, { zip: { names, modified: Date.now() } }, { onProgress });
      await this.#saveOne(
        zipped.then((w) => ("zip" in w ? w.zip : noFile())),
        where,
      );
      return;
    }
    const written = writeParts(client, ranges, {}, { onProgress }).then((w) => ("files" in w ? w.files : []));
    if (where !== undefined) {
      const files = (await written).map((bytes, i) => ({ name: names[i] ?? "part.pdf", bytes }));
      await this.#writeFolder(where, files);
      return;
    }
    for (const [i, name] of names.entries())
      await this.#saveOne(
        written.then((f) => f[i] ?? noFile()),
        name,
      );
  }

  /** Closes the dialog and the previews it opened. */
  async close(): Promise<void> {
    if (this.#state.progress !== undefined) return;
    const { source, merge } = this.#state;
    this.#set({ dialog: undefined, source: undefined, merge: [], images: [] });
    await Promise.all([source, ...merge].map((p) => this.#close(p)));
  }

  /** Writes `files` into `folder`, asking first when some names are taken: replace them, or keep both. */
  async #writeFolder(folder: Folder, files: readonly { name: string; bytes: Uint8Array }[]): Promise<void> {
    const taken = new Set(await folder.existing(files.map((f) => f.name)));
    const replace = taken.size === 0 ? true : await this.#ask(taken.size);
    if (replace === undefined) throw new Cancelled();
    for (const file of files) {
      const name = taken.has(file.name) && !replace ? await freeName(folder, file.name) : file.name;
      await folder.write(name, file.bytes);
    }
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

  /** Saves `bytes`, maybe still coming, as a new file named `name` (made safe); a dismissed dialog cancels. */
  async #saveOne(bytes: Promise<Uint8Array>, name: string): Promise<void> {
    const result = await this.#platform.saveAs(bytes, name.endsWith(".zip") ? name : downloadName(name));
    if (!result.saved) throw new Cancelled();
  }

  async #addToMerge(files: readonly PlatformFile[]): Promise<void> {
    const picked = files.map((file) => this.#picked(file));
    this.#set({ merge: [...this.#state.merge, ...picked] });
    await Promise.all(picked.map(async (p) => this.#replaceMergeFile(await this.#read(p, undefined))));
  }

  async #claims(): Promise<void> {
    try {
      const { client } = await this.#engine.view();
      const metadata = await client.metadata();
      this.#set({ claims: claimsOf(metadata.pdfa, metadata.pdfua) });
    } catch {
      this.#set({ claims: [] });
    }
  }

  async #splitFacts(): Promise<void> {
    const { client } = await this.#engine.view();
    const [outline, plan] = await Promise.all([
      client.outline().catch(() => []),
      client.planSave({}).catch(() => undefined),
    ]);
    const chapters = outline.flatMap((node) =>
      node.target === undefined ? [] : [{ title: node.title, page: node.target.page }],
    );
    this.#set({ chapters, signed: (plan?.signatures.length ?? 0) > 0 });
  }

  #picked(file: PlatformFile): Picked {
    return {
      key: `f${nextKey++}`,
      file,
      bytes: undefined,
      password: undefined,
      preview: undefined,
      shown: { name: file.name, size: file.size, status: "loading" },
    };
  }

  /** `picked` read and previewed, with `password` when it is given; how it failed otherwise. */
  async #read(picked: Picked, password: string | undefined): Promise<Picked> {
    const { name, size } = picked.file;
    try {
      const bytes = picked.bytes ?? (await picked.file.read(0, size));
      const { client } = await this.#engine.view();
      try {
        const preview = await previewSource(client, bytes, password === undefined ? {} : { password });
        const pages = preview.pages.map(({ width, height }) => ({ width, height }));
        return { ...picked, bytes, password, preview, shown: { name, size, status: "ready", pages } };
      } catch (error) {
        if (hasCode(error, "password-required") || hasCode(error, "wrong-password")) {
          return { ...picked, bytes, shown: { name, size, status: "password", wrong: password !== undefined } };
        }
        throw error;
      }
    } catch {
      return { ...picked, shown: { name, size, status: "unreadable", error: word("documents.file.unreadable.body") } };
    }
  }

  async #close(picked: Picked | undefined): Promise<void> {
    const key = picked?.preview?.key;
    if (key === undefined) return;
    const { client } = await this.#engine.view();
    await closeSource(client, key).catch(() => undefined);
  }

  #set(change: Partial<DocumentsState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** A picture's media type by its name, as From images takes it: PNG, else JPEG. */
function imageType(name: string): string {
  return /\.png$/i.test(name) ? "image/png" : "image/jpeg";
}

/** The handlers of the commands across files while Organise is the mode. */
export function documentsActions(documents: DocumentsController) {
  return {
    "pages.insert-from-file": () => void documents.openFromFile("from-file"),
    "pages.replace": () => void documents.openFromFile("replace"),
    "pages.extract": () => documents.openSplit("extract"),
    "document.split": () => documents.openSplit("split"),
    "document.merge": () => void documents.openMerge(),
    "document.merge-add-files": () => void documents.addMergeFiles(),
    "pages.insert-images": () => void documents.openImages(),
  } as const;
}

/** The dialogs across files. */
export function DocumentsDialogs({
  documents,
  organise,
  engine,
  fileName,
}: {
  documents: DocumentsController;
  organise: OrganiseController;
  engine: Engine;
  /** The open file's name, which new files are named from. */
  fileName: string;
}): ReactNode {
  const state = useStoreState(documents, (s) => s);
  const pages = useStoreState(engine, (s) => s.pages);
  const selectedIds = useStoreState(organise, (s) => s.selected);
  const selected = pages.flatMap((p, i) => (selectedIds.has(p.id) ? [i] : []));
  const labels = pages.map((p, i) => p.label ?? String(i + 1));
  const close = () => void documents.close();
  // The dialogs go with the document: their previews close with them.
  useEffect(() => () => void documents.close(), [documents]);
  const source = state.source;
  const drawSource = useCallback(
    (page: number, canvas: HTMLCanvasElement, signal: AbortSignal) => documents.draw(source, page, canvas, signal),
    [documents, source],
  );
  const name = downloadStem(fileName);
  const mergeFiles: MergeFile[] = state.merge.map((p) => ({
    key: p.key,
    source: p.shown,
    claims: claimsOf(p.preview?.pdfa, p.preview?.pdfua),
  }));
  const splitting = state.dialog === "split" || state.dialog === "extract";
  return (
    <>
      <PagesFromFileDialog
        isOpen={state.dialog === "from-file" || state.dialog === "replace"}
        onClose={close}
        mode={state.dialog === "replace" ? "replace" : "insert"}
        source={source?.shown}
        onChoose={() => void documents.chooseSource()}
        onPassword={(password) => void documents.sourcePassword(password)}
        draw={drawSource}
        labels={labels}
        selected={selected}
        onApply={(choice) => void documents.applyFromFile(choice)}
      />
      <MergeDialog
        isOpen={state.dialog === "merge"}
        onClose={close}
        current={{ name, claims: state.claims }}
        files={mergeFiles}
        onAdd={() => void documents.addMergeFiles()}
        onMove={(key, delta) => documents.moveMergeFile(key, delta)}
        onRemove={(key) => void documents.removeMergeFile(key)}
        onPassword={(key, password) => void documents.mergePassword(key, password)}
        onMerge={(choice) => void documents.merge(choice, name)}
      />
      <SplitDialog
        isOpen={splitting}
        onClose={close}
        start={state.dialog === "extract" ? "ranges" : "count"}
        labels={labels}
        selected={selected}
        chapters={state.chapters}
        stem={name}
        output={(parts) => documents.output(parts)}
        signed={state.signed}
        progress={state.progress}
        onSplit={(parts) => void documents.split(parts, name)}
      />
      <ImagesDialog documents={documents} images={state.dialog === "images" ? state.images : undefined} />
      <ExistsDialog ask={state.exists} />
    </>
  );
}

/** From images (O07): the pictures chosen, while its dialog is open (`images` undefined when it isn't). */
function ImagesDialog({
  documents,
  images,
}: {
  documents: DocumentsController;
  images: readonly PlatformFile[] | undefined;
}): ReactNode {
  return (
    <ImagesToPdfDialog
      isOpen={images !== undefined}
      files={images ?? []}
      onChooseFiles={() => void documents.chooseImages()}
      onMake={(choice) => void documents.makeImagesPdf(choice)}
      onClose={() => void documents.close()}
    />
  );
}

/** The question when files of the parts' names are in the chosen folder (O08): Replace, or Keep both. */
function ExistsDialog({ ask }: { ask: DocumentsState["exists"] }): ReactNode {
  const count = ask?.count ?? 0;
  return (
    <Dialog
      isOpen={ask !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) ask?.answer(undefined);
      }}
      onEscape={() => ask?.answer(undefined)}
      title={words(count, { one: "documents.exists.one", many: "documents.exists" })}
      description={word("documents.exists.body")}
      actions={
        <>
          <Button onPress={() => ask?.answer(undefined)}>{word("organise.move.cancel")}</Button>
          <Button onPress={() => ask?.answer(true)}>{word("documents.exists.replace")}</Button>
          <Button variant="primary" autoFocus onPress={() => ask?.answer(false)}>
            {word("documents.exists.keep")}
          </Button>
        </>
      }
    />
  );
}
