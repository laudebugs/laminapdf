// A file the platform's picker gave, as a `File` read whole: a document the person chose to put
// into the open one (a picture, a font, an attachment), not one the worker opens.

import type { PlatformFile } from "@laminapdf/ui";

/** `picked` as a `File`: its own when the picker gave one, else its bytes read. */
export async function pickedFile(picked: PlatformFile): Promise<File> {
  return picked.handle instanceof File
    ? picked.handle
    : new File([new Uint8Array(await picked.read(0, picked.size))], picked.name);
}
