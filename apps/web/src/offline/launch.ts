// Files the operating system hands the installed reader (C7b): "Open with Lamina PDF" on a PDF,
// where the browser supports the manifest's `file_handlers` (Chromium). The platform turns its
// launch queue into open requests; the first file of each opens like one chosen here.

import type { Platform, ReaderController } from "@laminapdf/ui";

/** Opens the first file of each launch the platform reports. */
export function openLaunchedFiles(
  platform: Pick<Platform, "onOpenRequest">,
  reader: Pick<ReaderController, "open">,
): () => void {
  return platform.onOpenRequest((files) => {
    const [file] = files;
    if (file !== undefined) void reader.open(file).catch((error: unknown) => reportError(error));
  });
}
