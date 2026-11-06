// The card the canvas holds while a whole file downloads (R02): when the host can't send ranges,
// nothing can show until the file is complete. A determinate bar with megabytes and the
// percentage when the size is known, an indeterminate one when it isn't, and Cancel.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { ProgressBar } from "../components/progress.js";
import { message } from "../messages.js";

/** Props of `WholeFileDownload`. */
export interface WholeFileDownloadProps {
  /** Bytes received so far. */
  loaded: number;
  /** The file's size in bytes, when the host said. */
  total?: number;
  /** Stops the download; the shell goes back to the home screen. */
  onCancel: () => void;
}

const MB = 1024 * 1024;
const megabytes = (bytes: number) => (bytes / MB).toFixed(1);

/** The whole-file download card. */
export function WholeFileDownload({ loaded, total, onCancel }: WholeFileDownloadProps): ReactNode {
  const known = total !== undefined && total > 0;
  const label = known
    ? message("loading.progress", { loaded: megabytes(loaded), total: megabytes(total) })
    : message("loading.progressUnknown", { loaded: megabytes(loaded) });
  return (
    <div className="paper-download">
      <h2 className="paper-download-title">{message("loading.title")}</h2>
      <p className="paper-download-why">{message("loading.why")}</p>
      <ProgressBar label={label} {...(known ? { value: Math.min(100, Math.round((loaded / total) * 100)) } : {})} />
      <Button onPress={onCancel}>{message("loading.cancel")}</Button>
    </div>
  );
}
