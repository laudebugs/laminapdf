// A file held in memory, as the reader opens one: a document made here that has never been
// saved (a merge, a comment summary). The reader reads it by range like any picked file.

import type { PlatformFile } from "@laminapdf/ui";

/** `bytes` as a file named `name`, which can't be saved in place. */
export function memoryFile(name: string, bytes: Uint8Array): PlatformFile {
  return {
    name,
    size: bytes.length,
    read: async (offset, length) => bytes.slice(offset, offset + length),
    handle: new File([bytes as BlobPart], name, { type: "application/pdf" }),
    writable: false,
  };
}
