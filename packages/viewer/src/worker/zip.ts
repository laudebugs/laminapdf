// A zip of whole files, for the parts Split and Export ranges write where the platform can't
// write a folder (the E2 plan §7): one download instead of one per part, built in one buffer in
// the worker. The format and its limits are ../zip/format.ts's.

import {
  directory,
  dosTime,
  LOCAL_HEADER,
  localHeader,
  MAX_BYTES,
  MAX_FILES,
  type ZipEntry,
  zipLimit,
} from "../zip/format.js";
import { crc32 } from "./crc32.js";

/** One file of a zip. */
export interface ZipFile {
  /** Its name in the zip: a plain file name, no folders. */
  readonly name: string;
  readonly bytes: Uint8Array;
}

/**
 * A zip holding `files` in order, each stored, dated `modified` (MS-DOS time, local, to two
 * seconds). Two files with one name are written as given: the caller names them apart.
 *
 * @throws {@link LaminaPdfError} `limit-exceeded` past 65,535 files or 4 GiB, which a zip without ZIP64 can't hold.
 */
export function zip(files: readonly ZipFile[], modified: Date): Uint8Array {
  if (files.length > MAX_FILES) throw zipLimit("zipFiles");
  const encoder = new TextEncoder();
  const entries: ZipEntry[] = [];
  let at = 0;
  for (const file of files) {
    const name = encoder.encode(file.name);
    entries.push({ name, size: file.bytes.length, crc: crc32(file.bytes), offset: at });
    at += LOCAL_HEADER + name.length + file.bytes.length;
    if (at > MAX_BYTES) throw zipLimit("zipBytes");
  }
  const when = dosTime(modified);
  const end = directory(entries, when, at);
  const out = new Uint8Array(at + end.length);
  entries.forEach((entry, i) => {
    const header = localHeader(entry, when);
    out.set(header, entry.offset);
    out.set((files[i] as ZipFile).bytes, entry.offset + header.length);
  });
  out.set(end, at);
  return out;
}
