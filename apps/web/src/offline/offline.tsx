// What the reader shows about the connection and updates (R15): the offline notice under the top
// bar while the browser is offline (dismissed until the next time it goes offline), "Offline" in
// the status bar, and, once a new version waits, a toast with Reload and Later and an "Update
// ready" status item that stays after Later. Reload is the only way a new version takes over.

import { message, OfflineNotice, StatusBarItem, type ToastQueue, useStoreState } from "@laminapdf/ui";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { onlineStore } from "./online.ts";
import type { AppUpdates } from "./updates.ts";

/** The window's connection. */
const online = onlineStore(window);

/** The offline notice and status items, and the update toast shown through `toasts`. */
export function useOffline(
  updates: AppUpdates,
  toasts: ToastQueue,
  documentOpen: boolean,
): { notice: ReactNode; status: ReactNode[] } {
  const isOnline = useStoreState(online, (s) => s);
  const [dismissed, setDismissed] = useState(false);
  // Each time the connection drops the notice shows again.
  useEffect(() => {
    if (isOnline) setDismissed(false);
  }, [isOnline]);
  const ready = useStoreState(updates, (s) => s.ready);
  // Read when the toast shows, so opening or closing a document never brings back one the user
  // dismissed with Later: the toast shows once per update, and the status item stays.
  const open = useRef(documentOpen);
  open.current = documentOpen;
  useEffect(() => {
    if (!ready) return;
    const toast = toasts.add({
      message: message(open.current ? "update.readyOpen" : "update.ready"),
      action: { label: message("update.reload"), onPress: () => updates.reload() },
      dismissLabel: message("update.later"),
      // It waits on the user's consent, so it never times out.
      timeout: Number.POSITIVE_INFINITY,
    });
    return () => toast.dismiss();
  }, [ready, toasts, updates]);
  const status: ReactNode[] = [];
  if (!isOnline)
    status.push(
      <StatusBarItem key="offline" icon="offline">
        {message("offline.status")}
      </StatusBarItem>,
    );
  if (ready) {
    status.push(
      <StatusBarItem key="update" icon="download" onPress={() => updates.reload()}>
        {message("update.status")}
      </StatusBarItem>,
    );
  }
  return {
    notice: isOnline || dismissed ? undefined : <OfflineNotice onDismiss={() => setDismissed(true)} />,
    status,
  };
}
