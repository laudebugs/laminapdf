// The files the reader opens from a link: each stands in the reader as a PlatformFile whose bytes
// only the document worker reads, and the address it came from is kept beside it, never in it.
// Apart from the session, so the address watcher and its tests need no viewer.

import type { PlatformFile } from "@laminapdf/ui";

/** Remote files by the PlatformFile that stands for each in the reader. */
const remotes = new WeakMap<PlatformFile, string>();

/** The file name a remote address shows: its last path segment, else its host. */
function remoteName(url: string): string {
  const { pathname, hostname } = new URL(url);
  const last = pathname
    .split("/")
    .filter((part) => part !== "")
    .at(-1);
  if (last === undefined) return hostname;
  try {
    return decodeURIComponent(last);
  } catch {
    return last;
  }
}

/** The PlatformFile that stands for the remote file at `url`. Only the document worker reads it. */
export function remoteFile(url: string): PlatformFile {
  const file: PlatformFile = {
    name: remoteName(url),
    size: 0,
    writable: false,
    read: () => Promise.reject(new Error("a remote file is read by the document worker")),
  };
  remotes.set(file, url);
  return file;
}

/** The address `file` stands for, when it is a remote file. */
export function remoteUrlOf(file: PlatformFile | undefined): string | undefined {
  return file === undefined ? undefined : remotes.get(file);
}
