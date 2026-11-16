// Export (O16): what the export dialog chose, made into files and saved, in the lazily loaded chunk.
// Every file is drawn or read in the document worker from the document as it is now, edits
// included. One file saves on its own; several go into a folder where the browser can write one,
// else into a zip that streams into a Blob a file at a time, so a long export never holds every
// page at once. Nothing is uploaded (ADR 0011).

import type { Platform, PlatformFolder, ToastQueue } from "@laminapdf/ui";
import { type ExportChoice, type ExportCounts, exportProgressTitle } from "@laminapdf/ui/dialogs";
import { laterMessage } from "@laminapdf/ui/editing";
import { type ViewerClient, ZipWriter } from "@laminapdf/viewer";
import type { Engine } from "./session.ts";

/** One file of an export. */
interface ExportFile {
  readonly name: string;
  readonly bytes: Uint8Array;
}

/** An export to run. */
export interface ExportJob {
  readonly choice: ExportChoice;
  /** The open file's name without `.pdf`, which the files are named after. */
  readonly stem: string;
  readonly client: ViewerClient;
  readonly platform: Platform;
  /** A folder chosen before the work, while the click still let the browser ask for one. */
  readonly folder: PlatformFolder | undefined;
  readonly signal: AbortSignal;
  readonly onProgress: (done: number, total: number) => void;
  /**
   * Page `page` (0-based) couldn't be drawn: resolves true to export the rest without it, false
   * to stop. The pages before it are kept either way.
   */
  readonly onFailed: (page: number) => Promise<boolean>;
}

/** How an export ended: the name it was saved as (a file, a zip or the folder), or undefined. */
export interface ExportOutcome {
  readonly saved: string | undefined;
  /** 0-based pages that couldn't be drawn and were left out. */
  readonly failed: readonly number[];
}

/** How many pages' pictures are read in one call to the document worker. */
const PICTURE_PAGES_AT_ONCE = 8;

/** The extension each page image format is saved with. */
const IMAGE_EXTENSIONS = { png: "png", jpeg: "jpg", tiff: "tif" } as const;

/** The extension each text format is saved with. */
const TEXT_EXTENSIONS = { text: "txt", markdown: "md", json: "json" } as const;

/** Where the files go: saved one by one as they come, then finished into the one result. */
interface Sink {
  add(file: ExportFile): Promise<void>;
  /** The name the export was saved as, or undefined when the person dismissed the save dialog. */
  finish(): Promise<string | undefined>;
}

const encoder = new TextEncoder();

/** The export under way, for the progress view. */
export interface ExportRunning {
  readonly choice: ExportChoice;
  /** "Exporting 24 pages as PNG", in the dialogs' words. */
  readonly title: string;
  readonly done: number;
  readonly total: number;
  readonly fileName: string | undefined;
  /** Pages that couldn't be drawn, and the question whether to go on without them. */
  readonly failed: readonly number[];
  readonly asking: ((goOn: boolean) => void) | undefined;
}

/** What starting an export needs of the reader. */
export interface ExportStart {
  readonly choice: ExportChoice;
  readonly counts: ExportCounts | undefined;
  /** The open file's name without `.pdf`. */
  readonly stem: string;
  readonly engine: Engine;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  /** The export under way, so another can't start and Cancel can stop it. */
  readonly controller: { current: AbortController | undefined };
  readonly setRunning: (update: (running: ExportRunning | undefined) => ExportRunning | undefined) => void;
}

/** Whether `choice` makes several files, which go into a folder where the browser can write one. */
function several(choice: ExportChoice, counts: ExportCounts | undefined): boolean {
  if (choice.kind === "links") return true;
  if (choice.kind === "pictures") return (counts?.pictures ?? 0) > 1;
  return choice.kind === "pages" && choice.format !== "tiff" && choice.pages.length > 1;
}

/**
 * Runs the export `start.choice` with its progress view, then a toast naming where it went. A
 * folder is asked for first, while the click that chose Export still lets the browser show its
 * picker.
 */
export async function startExport(start: ExportStart): Promise<void> {
  const { choice, platform, toasts, controller, setRunning } = start;
  let folder: PlatformFolder | undefined;
  if (several(choice, start.counts) && platform.capabilities.canWriteDirectory) {
    folder = await platform.chooseFolder?.();
    if (folder === undefined) return;
  }
  const abort = new AbortController();
  controller.current = abort;
  const update = (change: Partial<ExportRunning>) => setRunning((r) => (r === undefined ? r : { ...r, ...change }));
  // The title counts the pages chosen ("Exporting the pictures of 3 pages").
  const title = exportProgressTitle(choice, "pages" in choice ? choice.pages.length : 0);
  setRunning(() => ({ choice, title, done: 0, total: 1, fileName: folder?.name, failed: [], asking: undefined }));
  try {
    const { client } = await start.engine.view();
    const outcome = await runExport({
      choice,
      stem: start.stem,
      client,
      platform,
      folder,
      signal: abort.signal,
      onProgress: (done, total) => update({ done, total }),
      onFailed: (page) =>
        new Promise<boolean>((resolve) =>
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
        ),
    });
    if (outcome.saved !== undefined) toasts.add({ message: exportDoneMessage(outcome.saved, folder !== undefined) });
  } catch (error) {
    if (!abort.signal.aborted) {
      reportError(error);
      toasts.add({ message: laterMessage("export.failed") });
    }
  } finally {
    controller.current = undefined;
    setRunning(() => undefined);
  }
}

/** Runs `job`, reporting progress and failed pages, and saves what it made. */
export async function runExport(job: ExportJob): Promise<ExportOutcome> {
  const failed: number[] = [];
  const sink = sinkFor(job, expectedFiles(job.choice));
  await produce(job, sink, failed);
  job.signal.throwIfAborted();
  return { saved: await sink.finish(), failed };
}

/** How many files `choice` makes when nothing fails; pictures aren't known until they are read. */
function expectedFiles(choice: ExportChoice): number | undefined {
  switch (choice.kind) {
    case "pages":
      // One TIFF holds every page.
      return choice.format === "tiff" ? 1 : choice.pages.length;
    case "text":
      return 1;
    case "links":
      return 2;
    case "pictures":
      return undefined;
  }
}

/** Makes `job`'s files and hands each to `sink` as soon as it is made. */
async function produce(job: ExportJob, sink: Sink, failed: number[]): Promise<void> {
  const { choice, client, signal, stem } = job;
  switch (choice.kind) {
    case "pages": {
      if (choice.format === "tiff") {
        job.onProgress(0, 1);
        const tiff = await client.exportTiff(
          choice.pages,
          { dpi: choice.dpi, multiPage: true, annotations: choice.annotations },
          { signal },
        );
        const several = tiff.files.length > 1;
        for (const [i, bytes] of tiff.files.entries())
          await sink.add({ name: several ? `${stem}-page-${(choice.pages[i] ?? i) + 1}.tif` : `${stem}.tif`, bytes });
        job.onProgress(1, 1);
        return;
      }
      const extension = IMAGE_EXTENSIONS[choice.format];
      for (const [done, page] of choice.pages.entries()) {
        job.onProgress(done, choice.pages.length);
        try {
          const { image } = await client.exportPage(
            page,
            { format: choice.format, dpi: choice.dpi, annotations: choice.annotations },
            { signal },
          );
          const name = choice.pages.length === 1 ? `${stem}.${extension}` : `${stem}-page-${page + 1}.${extension}`;
          await sink.add({ name, bytes: image });
        } catch (error) {
          signal.throwIfAborted();
          reportError(error);
          failed.push(page);
          if (!(await job.onFailed(page))) return;
        }
      }
      job.onProgress(choice.pages.length, choice.pages.length);
      return;
    }
    case "text": {
      job.onProgress(0, 1);
      const { text } = await client.extractText(
        { format: choice.format, pages: choice.pages, readingOrder: choice.tagged ? "tagged" : "auto" },
        { signal },
      );
      await sink.add({ name: `${stem}.${TEXT_EXTENSIONS[choice.format]}`, bytes: encoder.encode(text) });
      job.onProgress(1, 1);
      return;
    }
    case "pictures": {
      // A few pages at a time: the worker answers with every picture of the pages it is asked
      // for, so a long file never holds all its pictures at once.
      const pages = choice.pages;
      for (let at = 0; at < pages.length; at += PICTURE_PAGES_AT_ONCE) {
        job.onProgress(at, pages.length);
        const { pictures } = await client.extractPictures(
          {
            pages: pages.slice(at, at + PICTURE_PAGES_AT_ONCE),
            keepOriginals: choice.keepOriginals,
            minimumSize: choice.minimumSize,
          },
          { signal },
        );
        for (const picture of pictures) await sink.add({ name: `${stem}-${picture.name}`, bytes: picture.bytes });
      }
      job.onProgress(pages.length, pages.length);
      return;
    }
    case "links": {
      job.onProgress(0, 1);
      const { links, bookmarks } = await client.linksAndBookmarks(choice.format, { signal });
      await sink.add({ name: `${stem}-links.${choice.format}`, bytes: encoder.encode(links) });
      await sink.add({ name: `${stem}-bookmarks.${choice.format}`, bytes: encoder.encode(bookmarks) });
      job.onProgress(1, 1);
      return;
    }
  }
}

/**
 * Where `job`'s files go: the chosen folder; one file saved on its own when exactly one is
 * expected; else a zip. Pictures, whose count isn't known first, wait to see whether they are one.
 */
function sinkFor(job: ExportJob, expected: number | undefined): Sink {
  if (job.folder !== undefined) return folderSink(job.folder);
  if (expected === 1) return singleSink(job.platform);
  if (expected === undefined) return waitingSink(job);
  return zipSink(job.platform, `${job.stem}-${zipSuffix(job.choice)}.zip`);
}

/** The part of the zip's name that says what is in it. */
function zipSuffix(choice: ExportChoice): string {
  return { pages: "pages", text: "text", pictures: "pictures", links: "links-and-bookmarks" }[choice.kind];
}

/** Saves the one file it gets on its own. */
function singleSink(platform: Platform): Sink {
  let file: ExportFile | undefined;
  return {
    add: async (f) => {
      file = f;
    },
    finish: async () =>
      file === undefined ? undefined : save(platform, new Blob([file.bytes as BlobPart]), file.name),
  };
}

/** Packs every file into a zip, streamed into a Blob as each comes, and saves it at the end. */
function zipSink(platform: Platform, name: string): Sink {
  const zip = new ZipWriter(new Date());
  return {
    add: async (file) => zip.add(file.name, file.bytes),
    finish: () => save(platform, zip.finish(), name),
  };
}

/** Holds the first file until a second comes: one file saves on its own, more go into a zip. */
function waitingSink(job: ExportJob): Sink {
  let first: ExportFile | undefined;
  let zip: Sink | undefined;
  return {
    add: async (file) => {
      if (zip !== undefined) return zip.add(file);
      if (first === undefined) {
        first = file;
        return;
      }
      zip = zipSink(job.platform, `${job.stem}-${zipSuffix(job.choice)}.zip`);
      await zip.add(first);
      await zip.add(file);
    },
    finish: async () => {
      if (zip !== undefined) return zip.finish();
      return first === undefined ? undefined : save(job.platform, new Blob([first.bytes as BlobPart]), first.name);
    },
  };
}

/** Writes each file into `folder` as it comes, with ` (2)`… added to a name already taken there. */
function folderSink(folder: PlatformFolder): Sink {
  return {
    add: async (file) => folder.write(await freeName(folder, file.name), file.bytes),
    finish: async () => folder.name,
  };
}

/** Saves `file` as `name`; undefined when the person dismissed the dialog. */
async function save(platform: Platform, file: Blob, name: string): Promise<string | undefined> {
  if (platform.saveExport === undefined) throw new Error("this platform can't save exports");
  const result = await platform.saveExport(file, name);
  return result.saved ? result.name : undefined;
}

/** `name`, or `name` with ` (2)`, ` (3)`… before its extension: the first that isn't in `folder`. */
async function freeName(folder: PlatformFolder, name: string): Promise<string> {
  if ((await folder.existing([name])).length === 0) return name;
  const dot = name.lastIndexOf(".");
  const [stem, extension] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  for (let n = 2; ; n++) {
    const candidate = `${stem} (${n})${extension}`;
    if ((await folder.existing([candidate])).length === 0) return candidate;
  }
}

/** The toast after an export: the file or zip it saved, or the folder it wrote into. */
function exportDoneMessage(saved: string, intoFolder: boolean): string {
  return intoFolder
    ? laterMessage("export.done.folder", { folder: saved })
    : laterMessage("export.done.file", { name: saved });
}
