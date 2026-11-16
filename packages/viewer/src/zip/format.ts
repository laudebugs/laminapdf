// The zip format the viewer writes, in one place for both writers: the worker's in-memory `zip`
// (Split and Export ranges) and the page's streaming ZipWriter (the export dialog). Files are
// stored as they are (method 0): PDFs and images are compressed already, so no deflate code, and
// a zip is the files' bytes plus 76 bytes and twice the name per file. The format is PKWARE's
// APPNOTE.TXT 6.3.10 (§4.3: local headers, then the central directory, then its end record);
// names are UTF-8 with general-purpose bit 11 set (§4.4.4, appendix D). Without ZIP64, which no
// export needs, every size and offset stays below 4 GiB and there are at most 65,535 files; past
// that the writers refuse with `limit-exceeded`.

import { LaminaPdfError } from "@laminapdf/sdk";
import { docsUrl } from "@laminapdf/sdk/explain";

/** The most files a zip without ZIP64 holds. */
export const MAX_FILES = 0xffff;
/** The most bytes a zip without ZIP64 addresses: every offset and size fits 32 bits. */
export const MAX_BYTES = 0xffffffff;

export const LOCAL_HEADER = 30;
const CENTRAL_HEADER = 46;
const END_RECORD = 22;
/** Version needed to extract: 2.0, for UTF-8 names (APPNOTE §4.4.3.2). */
const VERSION = 20;
/** General-purpose flag bit 11: the name is UTF-8. */
const UTF8 = 0x0800;

/** One stored file, as both of its headers describe it. */
export interface ZipEntry {
  /** Its name, UTF-8. */
  readonly name: Uint8Array;
  readonly size: number;
  readonly crc: number;
  /** Where its local header starts in the zip. */
  readonly offset: number;
}

/** MS-DOS time and date (APPNOTE §4.4.6). */
export interface DosTime {
  readonly time: number;
  readonly date: number;
}

/** The error a zip without ZIP64 gives past its limits: `limit` names which one. */
export function zipLimit(limit: "zipFiles" | "zipBytes"): LaminaPdfError {
  const message =
    limit === "zipFiles"
      ? `a zip without ZIP64 holds at most ${MAX_FILES} files`
      : "a zip without ZIP64 holds at most 4 GiB";
  return new LaminaPdfError({
    code: "limit-exceeded",
    kind: "limit",
    message,
    hint: "export fewer files at once, or into a folder",
    data: { limit, max: limit === "zipFiles" ? MAX_FILES : MAX_BYTES },
    docsUrl: docsUrl("limit-exceeded"),
  });
}

/** `entry`'s local header (§4.3.7), its name after it. */
export function localHeader(entry: ZipEntry, when: DosTime): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(LOCAL_HEADER + entry.name.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, 0x04034b50, true);
  view.setUint16(4, VERSION, true);
  view.setUint16(6, UTF8, true);
  view.setUint16(8, 0, true);
  view.setUint16(10, when.time, true);
  view.setUint16(12, when.date, true);
  view.setUint32(14, entry.crc, true);
  view.setUint32(18, entry.size, true);
  view.setUint32(22, entry.size, true);
  view.setUint16(26, entry.name.length, true);
  view.setUint16(28, 0, true);
  out.set(entry.name, LOCAL_HEADER);
  return out;
}

/** The central directory (§4.3.12) of `entries`, then its end record (§4.3.16); it starts at `at`. */
export function directory(entries: readonly ZipEntry[], when: DosTime, at: number): Uint8Array<ArrayBuffer> {
  const size = entries.reduce((sum, entry) => sum + CENTRAL_HEADER + entry.name.length, 0);
  const out = new Uint8Array(size + END_RECORD);
  const view = new DataView(out.buffer);
  let to = 0;
  for (const entry of entries) {
    view.setUint32(to, 0x02014b50, true);
    view.setUint16(to + 4, VERSION, true);
    view.setUint16(to + 6, VERSION, true);
    view.setUint16(to + 8, UTF8, true);
    view.setUint16(to + 10, 0, true);
    view.setUint16(to + 12, when.time, true);
    view.setUint16(to + 14, when.date, true);
    view.setUint32(to + 16, entry.crc, true);
    view.setUint32(to + 20, entry.size, true);
    view.setUint32(to + 24, entry.size, true);
    view.setUint16(to + 28, entry.name.length, true);
    // Extra field, comment, disk number, internal and external attributes: none.
    view.setUint32(to + 42, entry.offset, true);
    out.set(entry.name, to + CENTRAL_HEADER);
    to += CENTRAL_HEADER + entry.name.length;
  }
  view.setUint32(to, 0x06054b50, true);
  view.setUint16(to + 8, entries.length, true);
  view.setUint16(to + 10, entries.length, true);
  view.setUint32(to + 12, size, true);
  view.setUint32(to + 16, at, true);
  return out;
}

/** `when` as MS-DOS time and date (local, to two seconds), clamped to the format's 1980–2107. */
export function dosTime(when: Date): DosTime {
  const year = Math.min(2107, Math.max(1980, when.getFullYear()));
  if (year !== when.getFullYear()) return { time: 0, date: ((year - 1980) << 9) | (1 << 5) | 1 };
  return {
    time: (when.getHours() << 11) | (when.getMinutes() << 5) | (when.getSeconds() >> 1),
    date: ((year - 1980) << 9) | ((when.getMonth() + 1) << 5) | when.getDate(),
  };
}
