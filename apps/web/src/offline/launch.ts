// Files the operating system hands the installed reader: "Open with Lamina PDF" on a PDF,
// where the browser supports the manifest's `file_handlers` (Chromium). The platform turns its
// launch queue into open requests; one file opens like one chosen here, and several go to
// `several`, which asks whether to merge them (several.tsx).

import type { Platform, PlatformFile, ReaderController } from "@laminapdf/ui";

/** Opens the file of each launch the platform reports, or hands several to `several`. */
export function openLaunchedFiles(
  platform: Pick<Platform, "onOpenRequest">,
  reader: Pick<ReaderController, "open">,
  several: (files: PlatformFile[]) => void,
): () => void {
  return platform.onOpenRequest((files) => {
    if (files.length > 1) {
      several(files);
      return;
    }
    const [file] = files;
    if (file !== undefined) void reader.open(file).catch((error: unknown) => reportError(error));
  });
}
