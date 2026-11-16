// Export (O16): Export… in the More menu and the palette's export commands, each opening the export
// dialog on its own kind; then the progress view while the document worker draws or reads the
// files, and a toast naming where they went. The dialogs and the work load lazily, with the other
// dialogs (later.ts); what stays at start is this state and the commands' handlers.

import type { Platform, ToastQueue } from "@laminapdf/ui";
import type * as Dialogs from "@laminapdf/ui/dialogs";
import type { ExportChoice, ExportCounts, ExportKind } from "@laminapdf/ui/dialogs";
import { lazy, type ReactNode, Suspense, useRef, useState } from "react";
import { downloadStem } from "./download-name.ts";
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

/** The export commands the reader handles: Export… and the palette's four, each on its own kind. */
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

/**
 * What the file holds, for the dialog's cards. Pictures aren't counted (only decoding them all
 * could); a failed read leaves every count unknown, so no card is wrongly shut.
 */
function readCounts(engine: Engine): Promise<ExportCounts> {
  return engine
    .view()
    .then(({ client }) => client.exportSummary())
    .then(
      (summary) => ({ ...summary, pictures: undefined }),
      (error: unknown) => {
        reportError(error);
        return { pictures: undefined, links: undefined, bookmarks: undefined, tagged: false };
      },
    );
}

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
    void readCounts(engine).then(setCounts);
  };

  // The work loads with the dialog it follows; it runs in the lazily loaded chunk.
  const run = (choice: ExportChoice, document: OutputDocument) => {
    setOpen(false);
    void loadLater()
      .then(({ startExport }) =>
        startExport({
          choice,
          stem: downloadStem(document.fileName),
          engine,
          platform,
          toasts,
          controller,
          setRunning,
        }),
      )
      .catch((error: unknown) => reportError(error));
  };

  const cancel = () => {
    running?.asking?.(false);
    controller.current?.abort();
  };

  return {
    actions: Object.fromEntries(
      Object.entries(KINDS).map(([command, opening]) => [command, start(opening)]),
    ) as Exporting["actions"],
    dialogs: (document) =>
      document === undefined || (!open && running === undefined) ? null : (
        <ExportDialogs
          document={document}
          open={open}
          kind={kind}
          counts={counts}
          pages={pages}
          running={running}
          onExport={(choice) => run(choice, document)}
          onClose={() => setOpen(false)}
          onCancel={cancel}
        />
      ),
  };
}

/** The export dialog, then the progress view while an export runs. */
function ExportDialogs(props: {
  document: OutputDocument;
  open: boolean;
  kind: ExportKind | undefined;
  counts: ExportCounts | undefined;
  pages: readonly number[] | undefined;
  running: ExportRunning | undefined;
  onExport: (choice: ExportChoice) => void;
  onClose: () => void;
  onCancel: () => void;
}): ReactNode {
  const { document, running } = props;
  const current = document.viewer.getState().currentPage;
  return (
    <Suspense fallback={null}>
      <ExportDialog
        isOpen={props.open}
        kind={props.kind}
        pageCount={document.pages.length}
        currentPage={current}
        labels={document.labels}
        pageSize={document.pages[current]}
        counts={props.counts}
        pages={props.pages}
        onExport={props.onExport}
        onClose={props.onClose}
      />
      {running === undefined ? null : (
        <ExportProgressDialog
          isOpen
          title={running.title}
          done={running.done}
          total={running.total}
          // Pictures are read a few pages at a time, so their progress counts pages too.
          unit="pages"
          destination={running.destination}
          failed={running.asking === undefined ? undefined : running.failed}
          onCancel={props.onCancel}
          onExportRest={running.asking === undefined ? undefined : () => running.asking?.(true)}
          onClose={props.onCancel}
        />
      )}
    </Suspense>
  );
}
