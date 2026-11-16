// A zip of whole files, for the parts Split and Export ranges write where the platform can't
// write a folder (the E2 plan §7): one download instead of one per part. PDFs are compressed
// already, so each file is stored as it is (method 0): no deflate code, and the zip is the parts'
// bytes plus 76 bytes and twice the name per file. The format is PKWARE's APPNOTE.TXT 6.3.10
// (§4.3: local headers, then the central directory, then its end record); names are UTF-8 with
// general-purpose bit 11 set (§4.4.4, appendix D). Without ZIP64, which no part needs, every size
// and offset stays below 4 GiB and there are at most 65,535 files; past that it refuses.

import { crc32 } from "./crc32.js";

/** One file of a zip. */
export interface ZipFile {
  /** Its name in the zip: a plain file name, no folders. */
  readonly name: string;
  readonly bytes: Uint8Array;
}

/** The most files and bytes a zip without ZIP64 holds. */
const MAX_FILES = 0xffff;
const MAX_BYTES = 0xffffffff;

const LOCAL_HEADER = 30;
const CENTRAL_HEADER = 46;
const END_RECORD = 22;
/** Version needed to extract: 2.0, for UTF-8 names (APPNOTE §4.4.3.2). */
const VERSION = 20;
/** General-purpose flag bit 11: the name is UTF-8. */
const UTF8 = 0x0800;

/**
 * A zip holding `files` in order, each stored, dated `modified` (MS-DOS time, local, to two
 * seconds). Two files with one name are written as given: the caller names them apart.
 *
 * @throws a RangeError past 65,535 files or 4 GiB, which a zip without ZIP64 can't hold.
 */
export function zip(files: readonly ZipFile[], modified: Date): Uint8Array {
  if (files.length > MAX_FILES) throw new RangeError(`a zip holds at most ${MAX_FILES} files`);
  const encoder = new TextEncoder();
  const entries = files.map((file) => ({ name: encoder.encode(file.name), bytes: file.bytes, crc: crc32(file.bytes) }));
  const central = entries.reduce((sum, entry) => sum + CENTRAL_HEADER + entry.name.length, 0);
  const locals = entries.reduce((sum, entry) => sum + LOCAL_HEADER + entry.name.length + entry.bytes.length, 0);
  if (locals > MAX_BYTES || entries.some((entry) => entry.bytes.length > MAX_BYTES)) {
    throw new RangeError("a zip without ZIP64 holds at most 4 GiB");
  }
  const out = new Uint8Array(locals + central + END_RECORD);
  const view = new DataView(out.buffer);
  const { time, date } = dosTime(modified);
  const offsets: number[] = [];
  let at = 0;
  for (const entry of entries) {
    offsets.push(at);
    view.setUint32(at, 0x04034b50, true);
    view.setUint16(at + 4, VERSION, true);
    view.setUint16(at + 6, UTF8, true);
    view.setUint16(at + 8, 0, true);
    view.setUint16(at + 10, time, true);
    view.setUint16(at + 12, date, true);
    view.setUint32(at + 14, entry.crc, true);
    view.setUint32(at + 18, entry.bytes.length, true);
    view.setUint32(at + 22, entry.bytes.length, true);
    view.setUint16(at + 26, entry.name.length, true);
    view.setUint16(at + 28, 0, true);
    out.set(entry.name, at + LOCAL_HEADER);
    out.set(entry.bytes, at + LOCAL_HEADER + entry.name.length);
    at += LOCAL_HEADER + entry.name.length + entry.bytes.length;
  }
  const directory = at;
  entries.forEach((entry, i) => {
    view.setUint32(at, 0x02014b50, true);
    view.setUint16(at + 4, VERSION, true);
    view.setUint16(at + 6, VERSION, true);
    view.setUint16(at + 8, UTF8, true);
    view.setUint16(at + 10, 0, true);
    view.setUint16(at + 12, time, true);
    view.setUint16(at + 14, date, true);
    view.setUint32(at + 16, entry.crc, true);
    view.setUint32(at + 20, entry.bytes.length, true);
    view.setUint32(at + 24, entry.bytes.length, true);
    view.setUint16(at + 28, entry.name.length, true);
    // Extra field, comment, disk number, internal and external attributes: none.
    view.setUint32(at + 42, offsets[i] as number, true);
    out.set(entry.name, at + CENTRAL_HEADER);
    at += CENTRAL_HEADER + entry.name.length;
  });
  view.setUint32(at, 0x06054b50, true);
  view.setUint16(at + 8, entries.length, true);
  view.setUint16(at + 10, entries.length, true);
  view.setUint32(at + 12, at - directory, true);
  view.setUint32(at + 16, directory, true);
  return out;
}

/** `when` as MS-DOS time and date (APPNOTE §4.4.6), clamped to the format's 1980–2107. */
function dosTime(when: Date): { time: number; date: number } {
  const year = Math.min(2107, Math.max(1980, when.getFullYear()));
  if (year !== when.getFullYear()) return { time: 0, date: ((year - 1980) << 9) | (1 << 5) | 1 };
  return {
    time: (when.getHours() << 11) | (when.getMinutes() << 5) | (when.getSeconds() >> 1),
    date: ((year - 1980) << 9) | ((when.getMonth() + 1) << 5) | when.getDate(),
  };
}
