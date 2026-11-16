// The export dialog's zip, written on the page as each file arrives: each file's header and
// bytes are kept as Blob parts, so a zip of hundreds of exported pages need never sit in one
// ArrayBuffer, and the browser may keep the parts out of the JavaScript heap. The format and its
// limits are format.ts's.

import { LaminaPdfError } from "@laminapdf/sdk";
import { docsUrl } from "@laminapdf/sdk/explain";
import { crc32 } from "../worker/crc32.js";
import {
  type DosTime,
  directory,
  dosTime,
  LOCAL_HEADER,
  localHeader,
  MAX_BYTES,
  MAX_FILES,
  type ZipEntry,
  zipLimit,
} from "./format.js";

/**
 * A zip written a file at a time, each stored (not compressed again: PDFs and images are
 * compressed already), with UTF-8 names and no ZIP64: at most 65,535 files and 4 GiB.
 *
 * @example
 * ```ts
 * const writer = new ZipWriter(new Date());
 * for (const index of pages) {
 *   const page = await client.exportPage(index, { format: "png", dpi: 150 });
 *   writer.add(`page-${index + 1}.png`, page.image);
 * }
 * download(writer.finish());
 * ```
 */
export class ZipWriter {
  readonly #when: DosTime;
  readonly #entries: ZipEntry[] = [];
  readonly #names = new Set<string>();
  #parts: BlobPart[] | undefined = [];
  #size = 0;

  /** @param modified - the date every file is given (MS-DOS time: local, to two seconds). */
  constructor(modified: Date) {
    this.#when = dosTime(modified);
  }

  /**
   * Adds one file, stored (not compressed). Its bytes are copied into a Blob, so the caller may
   * let them go or reuse them.
   *
   * @param name - its name in the zip, UTF-8: a plain file name, or a path with `/`.
   * @param bytes - its contents.
   * @throws {@link LaminaPdfError} `limit-exceeded` past 65,535 files or 4 GiB (no ZIP64);
   *   `invalid-argument` for a name already in the zip, or once `finish` has run.
   */
  add(name: string, bytes: Uint8Array): void {
    const parts = this.#open();
    if (this.#names.has(name)) throw usage("a zip already holds a file of that name", "name");
    if (this.#entries.length >= MAX_FILES) throw zipLimit("zipFiles");
    const encoded = new TextEncoder().encode(name);
    const end = this.#size + LOCAL_HEADER + encoded.length + bytes.length;
    // The end record and its directory must fit as well, the last offset being 32 bits.
    if (end > MAX_BYTES) throw zipLimit("zipBytes");
    const entry = { name: encoded, size: bytes.length, crc: crc32(bytes), offset: this.#size };
    // Blob takes a copy of a typed array's bytes, whatever buffer type it views.
    parts.push(new Blob([localHeader(entry, this.#when), bytes as Uint8Array<ArrayBuffer>]));
    this.#entries.push(entry);
    this.#names.add(name);
    this.#size = end;
  }

  /**
   * The zip: the files added, in order, then their directory, as a Blob made of parts. The
   * writer takes no more files after.
   *
   * @throws {@link LaminaPdfError} `invalid-argument` when it has finished already.
   */
  finish(): Blob {
    const parts = this.#open();
    parts.push(directory(this.#entries, this.#when, this.#size));
    this.#parts = undefined;
    return new Blob(parts, { type: "application/zip" });
  }

  /** The bytes of the files added so far, headers included; the directory adds 46 bytes and the name per file, and 22. */
  get size(): number {
    return this.#size;
  }

  #open(): BlobPart[] {
    if (this.#parts === undefined) throw usage("the zip has finished: start a new ZipWriter", "writer");
    return this.#parts;
  }
}

/** A misuse of the writer, as Lamina PDF reports one. */
function usage(message: string, argument: string): LaminaPdfError {
  return new LaminaPdfError({
    code: "invalid-argument",
    kind: "usage",
    message,
    data: { argument },
    docsUrl: docsUrl("invalid-argument"),
  });
}
