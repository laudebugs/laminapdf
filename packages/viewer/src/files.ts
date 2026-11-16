// Pages across files, over a ViewerClient: previewing a PDF before its pages come in,
// writing pages of the open document as new files, and merging files into a new document. They
// are functions rather than the client's methods, so an app that loads its dialogs across files
// later loads these with them; `client.addSource`, which the edit log follows, is the client's.

import type { MergeOptions } from "@laminapdf/sdk";
import type { ViewerCallOptions, ViewerClient } from "./client.js";
import type {
  MergeFilesRequest,
  PartsOutput,
  PartsProgress,
  RenderedBitmap,
  SourcePreview,
  WrittenParts,
} from "./protocol.js";

/**
 * Opens a PDF to preview before its pages come in (Insert from a file, Replace, Merge): its pages'
 * sizes, the PDF/A and PDF/UA claims and title of its metadata, and a key that draws its pages
 * (`renderSource`) until `closeSource`. The password is sent with this call only.
 *
 * @throws `password-required` or `wrong-password` for an encrypted file; the errors of a file that doesn't open.
 */
export function previewSource(
  client: ViewerClient,
  bytes: Uint8Array,
  options?: { readonly password?: string },
  call?: ViewerCallOptions,
): Promise<SourcePreview> {
  const password = options?.password;
  return client.call("previewSource", password === undefined ? [bytes] : [bytes, password], call);
}

/** Page `page` of preview `key`, drawn at `scale`: a transferred bitmap, at background priority unless `call` says. */
export function renderSource(
  client: ViewerClient,
  key: number,
  page: number,
  scale: number,
  call?: ViewerCallOptions,
): Promise<RenderedBitmap> {
  return client.call("renderSource", [key, page, scale], { priority: "background", ...call });
}

/** Closes preview `key`. */
export async function closeSource(client: ViewerClient, key: number): Promise<void> {
  await client.call("closeSource", [key]);
}

/**
 * One new file per list of page IDs in `ranges` (pages of the open document as its edits leave
 * them, each list in the order given), each a whole file with its pages' annotations, links,
 * bookmarks, fields and labels; the document doesn't change. `output.zip` packs them into one zip
 * under its names. `onProgress` hears `{ done, total }` after each file.
 */
export async function writeParts(
  client: ViewerClient,
  ranges: readonly (readonly string[])[],
  output: PartsOutput,
  call?: ViewerCallOptions & { readonly onProgress?: (progress: PartsProgress) => void },
): Promise<WrittenParts> {
  const { onProgress, ...rest } = call ?? {};
  return client.call("writeParts", [ranges, output, await client.documentId()], {
    ...rest,
    ...(onProgress === undefined ? {} : { onProgress: (p: unknown) => onProgress(p as PartsProgress) }),
  });
}

/**
 * One new document of `files` one after another, with the open document first under the title
 * `open` when it is given (its pages as its edits leave them); the open document doesn't change.
 */
export async function mergeFiles(
  client: ViewerClient,
  files: MergeFilesRequest["files"],
  options: { readonly open?: string; readonly merge?: MergeOptions } = {},
  call?: ViewerCallOptions,
): Promise<Uint8Array> {
  const id = options.open === undefined ? 0 : await client.documentId();
  const request: MergeFilesRequest = {
    files,
    ...(options.open === undefined || id === 0 ? {} : { open: { id, title: options.open } }),
    ...(options.merge === undefined ? {} : { options: options.merge }),
  };
  return client.call("mergeFiles", [request], call);
}
