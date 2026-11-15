// File ▸ Print… (`file.print`, ⌘P) on the desktop (S05): the main process shows the OS's print
// dialog, then prints the pages this window's engine renders (src-tauri/src/print.rs). The page
// shown is the dialog's "Current page" on Windows and Linux. While pages spool, the status bar
// shows how many were sent, with Cancel printing, and reading goes on (non-modal). On Linux GTK
// prints from the main thread, so its own progress window shows instead and the events arrive as
// the job ends. A toast says how the job ended.
import { ProgressBar, StatusBarItem, type ToastQueue } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { type DesktopEngineClient, type DesktopShell, engineError } from "./engine/desktop-engine-client.ts";

/** Sent to this window when the pages are chosen and as each is drawn (src-tauri/src/print.rs). */
const PRINT_PROGRESS_EVENT = "print://progress";

/** How far the print has got. */
interface PrintProgress {
  readonly done: number;
  readonly total: number;
}

/** What `print_document` answers. */
type Printed = "printed" | "cancelled";

/** Failures that need no message: the user stopped the job, or a print is already running. */
const QUIET = new Set(["cancelled", "already-printing"]);

/** What printing needs of the window's engine (app.tsx's `DesktopEngine`). */
interface PrintingEngine {
  readonly shell: DesktopShell;
  readonly client: Pick<DesktopEngineClient, "current">;
  readonly viewer: Viewer;
}

/**
 * The `file.print` handler and the status-bar item it shows while pages spool.
 * @param engine - The window's engine, or `undefined` in a browser (no handler then).
 * @param toasts - Where the job's end is told.
 */
export function usePrint(
  engine: PrintingEngine | undefined,
  toasts: ToastQueue,
): { readonly actions: Readonly<Record<string, () => void>>; readonly status: ReactNode } {
  const shell = engine?.shell;
  const [progress, setProgress] = useState<PrintProgress>();
  const running = useRef(false);
  const sent = useRef(0);

  useEffect(() => {
    if (shell === undefined) return;
    const listening = shell.listen<PrintProgress>(PRINT_PROGRESS_EVENT, (next) => {
      // Only while this window's job runs.
      if (!running.current) return;
      sent.current = next.done;
      setProgress(next);
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
        sent.current = 0;
        // The page on screen, for the dialog's "Current page".
        const currentPage = engine?.viewer.getState().currentPage;
        void shell
          .invoke<Printed>("print_document", { doc: shown, currentPage })
          .then((printed) => {
            if (printed === "cancelled" && sent.current > 0) toasts.add({ message: "Printing cancelled" });
            if (printed === "printed") {
              const pages = sent.current === 1 ? "1 page" : `${sent.current} pages`;
              toasts.add({ message: `Sent ${pages} to the printer` });
            }
          })
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

  const status =
    progress === undefined ? null : (
      <span className="desktop-print-status">
        <ProgressBar
          label={`Printing · ${progress.done} of ${progress.total} sent`}
          value={progress.total === 0 ? 0 : (progress.done / progress.total) * 100}
        />
        <StatusBarItem
          icon="printer"
          onPress={() => {
            void shell?.invoke("cancel_print");
          }}
        >
          Cancel printing
        </StatusBarItem>
      </span>
    );
  return { actions, status };
}
