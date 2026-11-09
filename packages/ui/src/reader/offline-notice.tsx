// The offline notice (R15): a strip under the top bar while the browser is offline, saying the
// reader keeps working on this device's files and that a web address needs a connection. It is a
// polite status, so a screen reader hears it once; Dismiss hides it until the next time the
// connection drops. The app clears it on `online`, with no toast.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { IconButton } from "../components/button.js";
import { message } from "../messages.js";

/** Props of `OfflineNotice`. */
export interface OfflineNoticeProps {
  /** Dismiss was pressed. */
  onDismiss: () => void;
}

/** Says the reader is offline and what still works. */
export function OfflineNotice({ onDismiss }: OfflineNoticeProps): ReactNode {
  return (
    <div className="paper-offline-notice" role="status">
      <Icon name="offline" />
      <p>
        <strong>{message("offline.title")}</strong> {message("offline.body")}
      </p>
      <IconButton icon="close" size={26} label={message("offline.dismiss")} onPress={onDismiss} />
    </div>
  );
}
