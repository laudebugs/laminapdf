// The web platform's folder picker (File System Access: showDirectoryPicker), for writing the
// parts of a split into a folder the person picks. Loaded only when first asked for, so the app's
// startup script doesn't carry it.

import type { PlatformFolder } from "./types.js";
import type { WebScope } from "./web.js";

/** The folder picker's folder, or undefined when the user cancels. */
export async function chooseFolder(
  scope: Pick<WebScope, "showDirectoryPicker">,
  write: (handle: FileSystemFileHandle, bytes: Uint8Array) => Promise<void>,
): Promise<PlatformFolder | undefined> {
  let folder: FileSystemDirectoryHandle;
  try {
    folder = await (scope.showDirectoryPicker as NonNullable<WebScope["showDirectoryPicker"]>)({ mode: "readwrite" });
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return undefined;
    throw error;
  }
  return {
    name: folder.name,
    async existing(names) {
      const found = await Promise.all(
        names.map((name) =>
          folder.getFileHandle(name).then(
            () => true,
            (error: unknown) => {
              if (
                error instanceof DOMException &&
                (error.name === "NotFoundError" || error.name === "TypeMismatchError")
              ) {
                // No file of that name (or a folder, which a write refuses on its own).
                return error.name === "TypeMismatchError";
              }
              throw error;
            },
          ),
        ),
      );
      return names.filter((_, i) => found[i]);
    },
    async write(name, bytes) {
      await write(await folder.getFileHandle(name, { create: true }), bytes);
    },
  };
}
