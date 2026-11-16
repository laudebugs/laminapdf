// Export (O16): what the export dialog chose, made into files and saved, in the lazily loaded chunk.
// Every file is drawn or read in the document worker from the document as it is now, edits
// included. Where the files go is settled before the work, while the click that chose Export
// still lets the browser show its pickers: one file, named now, saves on its own; several go into
// a folder where the browser can write one, else into a zip that streams into a Blob a file at a
// time, so a long export never holds every page at once. Nothing is uploaded (ADR 0011).

import type { Platform, PlatformFolder, SaveResult, ToastQueue } from "@laminapdf/ui";
import { type ExportChoice, exportProgressTitle } from "@laminapdf/ui/dialogs";
import { laterMessage } from "@laminapdf/ui/editing";
import { type ViewerClient, ZipWriter } from "@laminapdf/viewer";
import { freeName } from "./free-name.ts";
import type { Engine } from "./session.ts";

/** One file of an export. */
interface ExportFile {
  readonly name: string;
  readonly bytes: Uint8Array;
}

/** Where an export's files go: one file of this name, a zip of this name, or a chosen folder. */
type Destination = { readonly file: string } | { readonly zip: string } | { readonly folder: PlatformFolder };

/** The export under way, for the progress view. */
export interface ExportRunning {
  readonly choice: ExportChoice;
  /** "Exporting 24 pages as PNG", in the dialogs' words. */
  readonly title: string;
  readonly done: number;
  readonly total: number;
  /** Where the files go, as the progress view says it. */
  readonly destination: { readonly file: string } | { readonly zip: string } | { readonly folder: string };
  /** Pages that couldn't be drawn, and the question whether to go on without them. */
  readonly failed: readonly number[];
  readonly asking: ((goOn: boolean) => void) | undefined;
}

/** What starting an export needs of the reader. */
export interface ExportStart {
  readonly choice: ExportChoice;
  /** The open file's name made safe, without `.pdf`. */
  readonly stem: string;
  readonly engine: Engine;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  /** The export under way, so another can't start and Cancel can stop it. */
  readonly controller: { current: AbortController | undefined };
  readonly setRunning: (update: (running: ExportRunning | undefined) => ExportRunning | undefined) => void;
}

/** How many pages' pictures are read in one call to the document worker. */
const PICTURE_PAGES_AT_ONCE = 8;

/** The extension each page image format is saved with. */
const IMAGE_EXTENSIONS = { png: "png", jpeg: "jpg", tiff: "tif" } as const;

/** The extension each text format is saved with. */
const TEXT_EXTENSIONS = { text: "txt", markdown: "md", json: "json" } as const;

/** The part of a zip's name that says what is in it. */
const ZIP_SUFFIXES = { pages: "pages", text: "text", pictures: "pictures", links: "links-and-bookmarks" } as const;

const encoder = new TextEncoder();

/** `choice` with each page once, in the order first chosen: a range may name a page twice. */
function once(choice: ExportChoice): ExportChoice {
  return "pages" in choice ? { ...choice, pages: [...new Set(choice.pages)] } : choice;
}

/**
 * The one file `choice` makes, named now: one page, a TIFF of every page, or the text. Pictures,
 * whose number isn't known first, and links with bookmarks, which are two files, make several.
 */
function singleName(choice: ExportChoice, stem: string): string | undefined {
  if (choice.kind === "text") return `${stem}.${TEXT_EXTENSIONS[choice.format]}`;
  if (choice.kind !== "pages") return undefined;
  if (choice.format === "tiff" || choice.pages.length === 1) return `${stem}.${IMAGE_EXTENSIONS[choice.format]}`;
  return undefined;
}

/**
 * Runs the export `start.choice` with its progress view, then a toast naming where it went. The
 * folder or the save dialog is asked for first, while the click that chose Export still lets the
 * browser show it; the file is written into the dialog's choice once it is made.
 */
export async function startExport(start: ExportStart): Promise<void> {
  const { platform, toasts, controller, setRunning, stem } = start;
  const choice = once(start.choice);
  const destination = await destinationFor(choice, stem, platform);
  if (destination === undefined) return;

  const abort = new AbortController();
  controller.current = abort;
  const made = deferred<Blob>();
  // A failed export rejects it; only a save waits on it, and a folder has none.
  made.promise.catch(() => undefined);
  const saving = "folder" in destination ? undefined : save(platform, made.promise, nameOf(destination));
  // A dismissed save dialog stops the work it was waiting for; a save that failed stops it too,
  // with the failure kept so it is reported, not taken for a cancel.
  let saveFailure: { readonly error: unknown } | undefined;
  void saving?.then(
    (result) => {
      if (!result.saved) abort.abort();
    },
    (error: unknown) => {
      saveFailure = { error };
      abort.abort();
    },
  );
  const update = (change: Partial<ExportRunning>) => setRunning((r) => (r === undefined ? r : { ...r, ...change }));
  const shown = "folder" in destination ? { folder: destination.folder.name } : destination;
  // The title counts the pages chosen ("Exporting the pictures of 3 pages").
  const title = exportProgressTitle(choice, "pages" in choice ? choice.pages.length : 0);
  setRunning(() => ({ choice, title, done: 0, total: 1, destination: shown, failed: [], asking: undefined }));
  try {
    const { client } = await start.engine.view();
    const sink = sinkFor(destination, abort.signal);
    await produce(
      {
        choice,
        stem,
        client,
        signal: abort.signal,
        onProgress: (done, total) => update({ done, total }),
        onFailed: (page) => asked(setRunning, update, page),
      },
      sink,
    );
    made.resolve(sink.finish());
    const result = saving === undefined ? ({ saved: true, name: nameOf(destination) } as const) : await saving;
    if (result.saved) toasts.add({ message: doneMessage(destination, result.name) });
  } catch (error) {
    made.reject(error);
    if (error instanceof NothingToExport) toasts.add({ message: laterMessage("export.none.pictures") });
    else if (!abort.signal.aborted || saveFailure !== undefined) {
      reportError(saveFailure?.error ?? error);
      toasts.add({ message: laterMessage("export.failed") });
    }
  } finally {
    controller.current = undefined;
    setRunning(() => undefined);
  }
}

/**
 * Where `choice`'s files go: its one file, named now; else a folder the person picks, where the
 * browser can write one (undefined when they cancel); else a zip.
 */
async function destinationFor(
  choice: ExportChoice,
  stem: string,
  platform: Platform,
): Promise<Destination | undefined> {
  const single = singleName(choice, stem);
  if (single !== undefined) return { file: single };
  const zip = { zip: `${stem}-${ZIP_SUFFIXES[choice.kind]}.zip` };
  if (!platform.capabilities.canWriteDirectory || platform.chooseFolder === undefined) return zip;
  try {
    const folder = await platform.chooseFolder();
    return folder === undefined ? undefined : { folder };
  } catch (error) {
    // The browser refused the picker (the click too long ago): a zip needs no folder.
    if (error instanceof DOMException && error.name === "SecurityError") return zip;
    throw error;
  }
}

/** The toast after an export saved as `saved`: the file or zip, or the folder it was written into. */
function doneMessage(destination: Destination, saved: string): string {
  return "folder" in destination
    ? laterMessage("export.done.folder", { folder: saved })
    : laterMessage("export.done.file", { name: saved });
}

/** Shows that page `page` couldn't be drawn and waits for the answer: export the rest, or stop. */
function asked(
  setRunning: ExportStart["setRunning"],
  update: (change: Partial<ExportRunning>) => void,
  page: number,
): Promise<boolean> {
  return new Promise((resolve) =>
    setRunning((r) =>
      r === undefined
        ? r
        : {
            ...r,
            failed: [...r.failed, page],
            asking: (goOn) => {
              update({ asking: undefined });
              resolve(goOn);
            },
          },
    ),
  );
}

/** The name a destination is saved as, or the folder's name. */
function nameOf(destination: Destination): string {
  return "folder" in destination ? destination.folder.name : "file" in destination ? destination.file : destination.zip;
}

/** Saves `file`, still being made, as `name`: the save dialog first, the writing when it's made. */
function save(platform: Platform, file: Promise<Blob>, name: string): Promise<SaveResult> {
  if (platform.saveExport === undefined) return Promise.reject(new Error("this platform can't save exports"));
  // The work's failure is reported by the export, not again by the save.
  file.catch(() => undefined);
  return platform.saveExport(file, name);
}

/** A promise with its resolve and reject at hand. */
function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T | Promise<T>) => void;
  reject: (error: unknown) => void;
} {
  let resolve: (value: T | Promise<T>) => void = () => undefined;
  let reject: (error: unknown) => void = () => undefined;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** The pages chosen hold no pictures: nothing to save, which the person is told. */
class NothingToExport extends Error {}

/** `answer`, or a rejection as soon as `signal` aborts: a question on screen goes when the export stops. */
function untilAborted<T>(answer: Promise<T>, signal: AbortSignal): Promise<T> {
  return new Promise((resolve, reject) => {
    const stop = () => reject(signal.reason);
    if (signal.aborted) return stop();
    signal.addEventListener("abort", stop, { once: true });
    answer.then(resolve, reject).finally(() => signal.removeEventListener("abort", stop));
  });
}

/** What making an export's files needs. */
interface Job {
  readonly choice: ExportChoice;
  readonly stem: string;
  readonly client: ViewerClient;
  readonly signal: AbortSignal;
  readonly onProgress: (done: number, total: number) => void;
  /** Page `page` (0-based) couldn't be drawn: resolves true to export the rest without it. */
  readonly onFailed: (page: number) => Promise<boolean>;
}

/** Where the files go as they are made, and the one file (or zip) they make at the end. */
interface Sink {
  add(file: ExportFile): Promise<void>;
  /** The file to save: the one file, or the zip; for a folder, nothing (its files are written). */
  finish(): Blob;
}

/** The sink for `destination`; a folder stops taking files once the export is cancelled. */
function sinkFor(destination: Destination, signal: AbortSignal): Sink {
  if ("folder" in destination) {
    const { folder } = destination;
    return {
      add: async (file) => {
        signal.throwIfAborted();
        await folder.write(await freeName(folder, file.name), file.bytes);
      },
      finish: () => new Blob([]),
    };
  }
  if ("file" in destination) {
    let one: Blob | undefined;
    return {
      add: async (file) => {
        if (one !== undefined) throw new Error("an export to one file made more than one");
        one = new Blob([file.bytes as BlobPart]);
      },
      finish: () => {
        if (one === undefined) throw new Error("the export made no file");
        return one;
      },
    };
  }
  const zip = new ZipWriter(new Date());
  return { add: async (file) => zip.add(file.name, file.bytes), finish: () => zip.finish() };
}

/** Makes `job`'s files and hands each to `sink` as soon as it is made. */
function produce(job: Job, sink: Sink): Promise<void> {
  const { choice } = job;
  switch (choice.kind) {
    case "pages":
      return choice.format === "tiff" ? tiff(job, choice, sink) : pageImages(job, choice, sink);
    case "text":
      return text(job, choice, sink);
    case "pictures":
      return pictures(job, choice, sink);
    case "links":
      return linksAndBookmarks(job, choice, sink);
  }
}

type Choice<K extends ExportChoice["kind"]> = Extract<ExportChoice, { kind: K }>;

/** Every page in one TIFF, drawn in one call to the worker. */
async function tiff(job: Job, choice: Choice<"pages">, sink: Sink): Promise<void> {
  job.onProgress(0, 1);
  const { files } = await job.client.exportTiff(
    choice.pages,
    { dpi: choice.dpi, multiPage: true, annotations: choice.annotations },
    { signal: job.signal },
  );
  // Past the export's memory budget the worker writes a TIFF a page, which one file can't hold.
  if (files.length !== 1) throw new Error("these pages are too large to export as one TIFF");
  await sink.add({ name: `${job.stem}.tif`, bytes: files[0] as Uint8Array });
  job.onProgress(1, 1);
}

/** A PNG or JPEG a page, each drawn on its own; a page that fails asks whether to go on without it. */
async function pageImages(job: Job, choice: Choice<"pages">, sink: Sink): Promise<void> {
  const extension = IMAGE_EXTENSIONS[choice.format];
  const one = choice.pages.length === 1;
  for (const [done, page] of choice.pages.entries()) {
    job.onProgress(done, choice.pages.length);
    try {
      const { image } = await job.client.exportPage(
        page,
        { format: choice.format === "jpeg" ? "jpeg" : "png", dpi: choice.dpi, annotations: choice.annotations },
        { signal: job.signal },
      );
      await sink.add({
        name: one ? `${job.stem}.${extension}` : `${job.stem}-page-${page + 1}.${extension}`,
        bytes: image,
      });
    } catch (error) {
      job.signal.throwIfAborted();
      // With one page there is no rest to export.
      if (one) throw error;
      reportError(error);
      if (!(await untilAborted(job.onFailed(page), job.signal))) throw error;
    }
  }
  job.onProgress(choice.pages.length, choice.pages.length);
}

/** The pages' text, as one file. */
async function text(job: Job, choice: Choice<"text">, sink: Sink): Promise<void> {
  job.onProgress(0, 1);
  const { text } = await job.client.extractText(
    { format: choice.format, pages: choice.pages, readingOrder: choice.tagged ? "tagged" : "auto" },
    { signal: job.signal },
  );
  await sink.add({ name: `${job.stem}.${TEXT_EXTENSIONS[choice.format]}`, bytes: encoder.encode(text) });
  job.onProgress(1, 1);
}

/**
 * The pages' pictures, a few pages at a time: the worker answers with every picture of the pages
 * it is asked for, so a long file never holds all its pictures at once.
 */
async function pictures(job: Job, choice: Choice<"pictures">, sink: Sink): Promise<void> {
  const { pages } = choice;
  let written = 0;
  for (let at = 0; at < pages.length; at += PICTURE_PAGES_AT_ONCE) {
    job.onProgress(at, pages.length);
    const read = await job.client.extractPictures(
      {
        pages: pages.slice(at, at + PICTURE_PAGES_AT_ONCE),
        keepOriginals: choice.keepOriginals,
        minimumSize: choice.minimumSize,
      },
      { signal: job.signal },
    );
    for (const picture of read.pictures) {
      job.signal.throwIfAborted();
      await sink.add({ name: `${job.stem}-${picture.name}`, bytes: picture.bytes });
      written++;
    }
  }
  // Pictures aren't counted before the export, so a page range with none is found here.
  if (written === 0) throw new NothingToExport();
  job.onProgress(pages.length, pages.length);
}

/** The file's links and its bookmarks, as two files. */
async function linksAndBookmarks(job: Job, choice: Choice<"links">, sink: Sink): Promise<void> {
  job.onProgress(0, 1);
  const { links, bookmarks } = await job.client.linksAndBookmarks(choice.format, { signal: job.signal });
  await sink.add({ name: `${job.stem}-links.${choice.format}`, bytes: encoder.encode(links) });
  await sink.add({ name: `${job.stem}-bookmarks.${choice.format}`, bytes: encoder.encode(bookmarks) });
  job.onProgress(1, 1);
}
