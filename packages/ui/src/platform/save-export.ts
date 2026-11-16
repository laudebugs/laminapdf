// Saving an export (pictures of pages, text, JSON, CSV or a zip of them) on the web: a save dialog
// typed by the name's extension where the browser has one, else a download. Loaded with the first
// export, so the reader's start doesn't carry it.

import type { SaveResult } from "./types.js";

/** What saving needs of web.ts: the download, the write to a chosen file, and the dismissed dialog's test. */
export interface SaveTools<Scope> {
  download(scope: Scope, file: Blob, name: string): void;
  write(handle: FileSystemFileHandle, bytes: Uint8Array | Blob): Promise<void>;
  isAbort(error: unknown): boolean;
}

/** The media type of an export by its extension; the save dialog offers it as the file's type. */
const EXPORT_TYPES: Readonly<Record<string, string>> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  jp2: "image/jp2",
  tif: "image/tiff",
  tiff: "image/tiff",
  txt: "text/plain",
  md: "text/markdown",
  json: "application/json",
  csv: "text/csv",
  zip: "application/zip",
};

/**
 * Saves `file`, maybe still being made, as `suggestedName`. The save dialog opens at once, while
 * the click that started the export still lets the browser show it, and the file is written when
 * it's ready. A browser that refuses the dialog all the same (the click too long ago) downloads it.
 */
export async function saveExport<
  Scope extends { showSaveFilePicker?: (options?: object) => Promise<FileSystemFileHandle> },
>(scope: Scope, file: Blob | Promise<Blob>, suggestedName: string, tools: SaveTools<Scope>): Promise<SaveResult> {
  const { download, write, isAbort } = tools;
  const extension = suggestedName.split(".").at(-1)?.toLowerCase() ?? "";
  const type = EXPORT_TYPES[extension] ?? "application/octet-stream";
  const coming = Promise.resolve(file).then((made) => (made.type === type ? made : new Blob([made], { type })));
  const downloaded = async (): Promise<SaveResult> => {
    download(scope, await coming, suggestedName);
    return { saved: true, name: suggestedName };
  };
  if (scope.showSaveFilePicker === undefined) return downloaded();
  // A dismissed dialog leaves the file's failure, if any, to nobody.
  coming.catch(() => undefined);
  let handle: FileSystemFileHandle;
  try {
    const types = [{ description: extension.toUpperCase(), accept: { [type]: [`.${extension}`] } }];
    handle = await scope.showSaveFilePicker({ suggestedName, types });
  } catch (error) {
    if (isAbort(error)) return { saved: false, reason: "cancelled" };
    if (error instanceof DOMException && error.name === "SecurityError") return downloaded();
    throw error;
  }
  await write(handle, await coming);
  return { saved: true, name: handle.name };
}
