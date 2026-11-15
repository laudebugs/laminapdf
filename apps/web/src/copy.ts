// Copying text in the reader: every Copy button (an open failure's details, the repairs, a web
// link, the network report, the document's properties) says it worked with the same short toast,
// "Copied", so a press that changes nothing on screen still shows it did something.

import { message, type Platform, type ToastQueue } from "@laminapdf/ui";

/** Copies `text` to the clipboard, then says so. */
export type Copy = (text: string) => void;

/** The reader's `Copy`: `platform` copies, `toasts` says so, and a copy that fails is reported. */
export function copier(platform: Platform, toasts: ToastQueue): Copy {
  return (text) =>
    void platform.copyText(text).then(() => {
      toasts.add({ message: message("shell.copied") });
    }, reportError);
}
