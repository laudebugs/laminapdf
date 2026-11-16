// Export (O16): Export… in the More menu and the palette's export commands, each opening the export
// dialog on its own kind; then the progress view while the document worker draws or reads the
// files, and a toast naming where they went. The dialogs and the work load lazily, with the other
// dialogs (later.ts); what stays at start is this state and the commands' handlers.

import type { Platform, ToastQueue } from "@laminapdf/ui";
import type * as Dialogs from "@laminapdf/ui/dialogs";
import type { ExportChoice, ExportCounts, ExportKind } from "@laminapdf/ui/dialogs";
import { lazy, type ReactNode, Suspense, useRef, useState } from "react";
import type { ExportRunning } from "./export-run.ts";
import type { OutputDocument } from "./output.tsx";
import type { Engine } from "./session.ts";

/** The commands this module handles, by the kind each opens the dialog on. */
const KINDS = {
  "file.export": undefined,
  "pages.export-images": "pages",
  "file.export-text": "text",
  "file.export-pictures": "pictures",
  "file.export-links-and-bookmarks": "links",
} as const satisfies Record<string, ExportKind | undefined>;

export type ExportCommand = keyof typeof KINDS;

/** What the hook gives the app: the commands' handlers (each takes the palette's page range), and the dialogs. */
export interface Exporting {
  readonly actions: Readonly<Record<ExportCommand, (args?: unknown) => void>>;
  readonly dialogs: (document: OutputDocument | undefined) => ReactNode;
}

const loadLater = () => import("./later.ts");
const nothing = (error: unknown) => {
  reportError(error);
  return { default: () => null };
};
const ExportDialog = lazy<typeof Dialogs.ExportDialog>(() =>
  loadLater().then((m) => ({ default: m.ExportDialog }), nothing),
);
const ExportProgressDialog = lazy<typeof Dialogs.ExportProgressDialog>(() =>
  loadLater().then((m) => ({ default: m.ExportProgressDialog }), nothing),
);

/** Export… and the palette's export commands, for the open document. */
export function useExport(engine: Engine, platform: Platform, toasts: ToastQueue): Exporting {
  const [open, setOpen] = useState(false);
  const [kind, setKind] = useState<ExportKind>();
  const [pages, setPages] = useState<readonly number[]>();
  const [counts, setCounts] = useState<ExportCounts>();
  const [running, setRunning] = useState<ExportRunning>();
  const controller = useRef<AbortController | undefined>(undefined);

  const start = (opening: ExportKind | undefined) => (args?: unknown) => {
    if (controller.current !== undefined || platform.saveExport === undefined) return;
    setKind(opening);
    // The palette's page range (0-based), when one was typed.
    setPages(Array.isArray(args) ? (args as readonly number[]) : undefined);
    setCounts(undefined);
    setOpen(true);
    void engine.view().then(
      ({ client }) => client.exportSummary().then(setCounts),
      (error: unknown) => reportError(error),
    );
  };

  // The work loads with the dialog it follows; it runs in the lazily loaded chunk.
  const run = (choice: ExportChoice, document: OutputDocument) => {
    setOpen(false);
    void loadLater().then(({ startExport }) =>
      startExport({
        choice,
        counts,
        stem: document.fileName.replace(/\.pdf$/i, ""),
        engine,
        platform,
        toasts,
        controller,
        setRunning,
      }),
    );
  };

  const cancel = () => {
    running?.asking?.(false);
    controller.current?.abort();
  };

  return {
    actions: {
      "file.export": start(KINDS["file.export"]),
      "pages.export-images": start(KINDS["pages.export-images"]),
      "file.export-text": start(KINDS["file.export-text"]),
      "file.export-pictures": start(KINDS["file.export-pictures"]),
      "file.export-links-and-bookmarks": start(KINDS["file.export-links-and-bookmarks"]),
    },
    dialogs: (document) =>
      document === undefined || (!open && running === undefined) ? null : (
        <Suspense fallback={null}>
          <ExportDialog
            isOpen={open}
            kind={kind}
            pageCount={document.pages.length}
            currentPage={document.viewer.getState().currentPage}
            labels={document.labels}
            pageSize={document.pages[document.viewer.getState().currentPage]}
            counts={counts}
            pages={pages}
            onExport={(choice) => void run(choice, document)}
            onClose={() => setOpen(false)}
          />
          {running === undefined ? null : (
            <ExportProgressDialog
              isOpen
              title={running.title}
              done={running.done}
              total={running.total}
              // Pictures are read a few pages at a time, so their progress counts pages too.
              unit="pages"
              fileName={running.fileName}
              failed={running.asking === undefined ? undefined : running.failed}
              onCancel={cancel}
              onExportRest={running.asking === undefined ? undefined : () => running.asking?.(true)}
              onClose={cancel}
            />
          )}
        </Suspense>
      ),
  };
}
