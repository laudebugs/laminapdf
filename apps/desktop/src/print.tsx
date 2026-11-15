// File ▸ Print… (`file.print`, ⌘P) on the desktop: the main process shows the OS's print dialog,
// then prints the pages this window's engine renders (src-tauri/src/print.rs). The page shown is
// the dialog's "Current page". On macOS and Windows the pages spool while the webview runs, so it
// shows their progress, whose Cancel stops the job; on Linux GTK's own progress window does, and the
// progress events arrive only as the job ends.
import type { ToastQueue } from "@laminapdf/ui";
import { PrintProgressDialog } from "@laminapdf/ui/dialogs";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { type DesktopEngineClient, type DesktopShell, engineError } from "./engine/desktop-engine-client.ts";

/** Sent to this window as each page is drawn (src-tauri/src/print.rs). */
export const PRINT_PROGRESS_EVENT = "print://progress";

/** How far the print has got. */
interface PrintProgress {
  readonly done: number;
  readonly total: number;
}

/** What `print_document` answers. */
type Printed = "printed" | "cancelled";

/** Failures that need no message: the user stopped the job, or a print is already running. */
const QUIET = new Set(["cancelled", "printing"]);

/** What printing needs of the window's engine (app.tsx's `DesktopEngine`). */
interface PrintingEngine {
  readonly shell: DesktopShell;
  readonly client: Pick<DesktopEngineClient, "current">;
  readonly viewer: Viewer;
}

/**
 * The `file.print` handler and the progress dialog it shows while pages spool.
 * @param engine - The window's engine, or `undefined` in a browser (no handler then).
 * @param toasts - Where a failed print is reported.
 */
export function usePrint(
  engine: PrintingEngine | undefined,
  toasts: ToastQueue,
): { readonly actions: Readonly<Record<string, () => void>>; readonly dialog: ReactNode } {
  const shell = engine?.shell;
  const [progress, setProgress] = useState<{ ready: number; total: number }>();
  const running = useRef(false);

  useEffect(() => {
    if (shell === undefined) return;
    const listening = shell.listen<PrintProgress>(PRINT_PROGRESS_EVENT, ({ done, total }) => {
      // Only while this window's job runs, and gone once every page is drawn.
      if (running.current) setProgress(done < total ? { ready: done, total } : undefined);
    });
    return () => void listening.then((stop) => stop());
  }, [shell]);

  const actions = useMemo(() => {
    if (shell === undefined) return {};
    return {
      "file.print": () => {
        const shown = engine?.client.current?.doc;
        if (shown === undefined || running.current) return;
        running.current = true;
        // The page on screen, for the dialog's "Current page".
        const currentPage = engine?.viewer.getState().currentPage;
        void shell
          .invoke<Printed>("print_document", { doc: shown, currentPage })
          .catch((error: unknown) => {
            const failure = engineError(error);
            if (!QUIET.has(failure.code)) toasts.add({ message: failure.message });
          })
          .finally(() => {
            running.current = false;
            setProgress(undefined);
          });
      },
    };
  }, [shell, engine, toasts]);

  const dialog = (
    <PrintProgressDialog
      progress={progress}
      onCancel={() => {
        void shell?.invoke("cancel_print");
      }}
    />
  );
  return { actions, dialog };
}
