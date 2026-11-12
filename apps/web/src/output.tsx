// The reader's output (R08, R09): Print… (`file.print`, Ctrl+P), Download a copy (`file.save-copy`)
// and Document properties (`file.properties`), their dialogs, and the toast a download ends with.
// Printing draws each page in the document worker and hands them to the platform one at a time,
// which prints them from a hidden frame; Cancel stops it between pages. Download saves the bytes as
// opened. Nothing is sent anywhere: both are local (ADR 0011).

import { Button, Dialog, message, type Platform, type ToastQueue } from "@laminapdf/ui";
import type * as Dialogs from "@laminapdf/ui/dialogs";
import type { PrintChoice, PropertiesState } from "@laminapdf/ui/dialogs";
import type { Viewer } from "@laminapdf/viewer";
import { lazy, type ReactNode, Suspense, useRef, useState } from "react";
import { downloadName } from "./download-name.ts";
import type { PageInfo } from "./outline.ts";
import type { EditablePropertiesDialog } from "./properties-editing.tsx";
import type { DocumentProperties, Engine } from "./session.ts";

/** The commands this module handles. */
export type OutputCommand = "file.print" | "file.save-copy" | "file.properties";

/** What the hook gives the app: the commands' handlers, and the dialogs to render. */
export interface Output {
  readonly actions: Readonly<Record<OutputCommand, () => void>>;
  readonly dialogs: ReactNode;
}

/** What the open document is, for the dialogs. */
export interface OutputDocument {
  readonly fileName: string;
  readonly pages: readonly PageInfo[];
  /** Each page's label, when the document labels its pages. */
  readonly labels: readonly string[] | undefined;
  readonly viewer: Viewer;
}

/** A failure to show, by the action that failed. */
type Problem = "print" | "download";

/** At 300 dpi, the most pixels a whole print may draw (about 120 US Letter pages); more prints at 150. */
const MAX_HIGH_QUALITY_PIXELS = 1_000_000_000;

/**
 * The resolution `choice` prints at: High quality lowered to Standard when its pages would draw
 * more than MAX_HIGH_QUALITY_PIXELS, since the frame holds every page's image until it prints.
 */
export function printDpi(choice: PrintChoice, pages: readonly PageInfo[]): { dpi: number; lowered: boolean } {
  if (choice.dpi !== 300) return { dpi: choice.dpi, lowered: false };
  const perPoint = (300 / 72) ** 2;
  const pixels = choice.pages.reduce((sum, i) => sum + (pages[i]?.width ?? 0) * (pages[i]?.height ?? 0) * perPoint, 0);
  return pixels > MAX_HIGH_QUALITY_PIXELS ? { dpi: 150, lowered: true } : { dpi: 300, lowered: false };
}

// The dialogs load once a document is open, not at start (#380). One that can't load (a lost
// connection) is reported and shows nothing; the reader carries on without it.
const loadDialogs = () => import("./later.ts");
const nothing = (error: unknown) => {
  reportError(error);
  return { default: () => null };
};
const PrintDialog = lazy<typeof Dialogs.PrintDialog>(() =>
  loadDialogs().then((m) => ({ default: m.PrintDialog }), nothing),
);
const PrintProgressDialog = lazy<typeof Dialogs.PrintProgressDialog>(() =>
  loadDialogs().then((m) => ({ default: m.PrintProgressDialog }), nothing),
);
const LazyEditablePropertiesDialog = lazy<typeof EditablePropertiesDialog>(() =>
  loadDialogs().then((m) => ({ default: m.EditablePropertiesDialog }), nothing),
);

/** Whether `error` is a stopped print, which needs no message. */
const aborted = (error: unknown) => error instanceof DOMException && error.name === "AbortError";

/** Print… and its progress: the dialog, then the pages drawn one by one, then the print window. */
function usePrint(engine: Engine, platform: Platform, onProblem: (problem: Problem) => void) {
  const [open, setOpen] = useState(false);
  const [encryption, setEncryption] = useState<DocumentProperties["encryption"]>();
  const [progress, setProgress] = useState<{ ready: number; total: number }>();
  const [lowered, setLowered] = useState(false);
  const running = useRef<AbortController | undefined>(undefined);

  const start = () => {
    if (running.current !== undefined) return;
    // The author's permissions decide the dialog's notice; a file whose properties can't be read
    // prints as one that restricts nothing.
    void engine.properties().then(
      (properties) => {
        setEncryption(properties.encryption);
        setOpen(true);
      },
      () => {
        setEncryption(undefined);
        setOpen(true);
      },
    );
  };

  const print = async (choice: PrintChoice, pages: readonly PageInfo[]) => {
    setOpen(false);
    const { dpi, lowered } = printDpi(choice, pages);
    setLowered(lowered);
    const controller = new AbortController();
    running.current = controller;
    setProgress({ ready: 0, total: choice.pages.length });
    try {
      await platform.print({
        pages: choice.pages,
        scale: choice.scale,
        signal: controller.signal,
        render: (page, signal) => engine.printPage(page, { dpi, annotations: choice.annotations }, signal),
        // Every page ready: the print window opens, and the progress dialog makes way for it.
        onProgress: (ready, total) => setProgress(ready < total ? { ready, total } : undefined),
      });
    } catch (error) {
      if (!aborted(error) && !controller.signal.aborted) {
        onProblem("print");
        reportError(error);
      }
    } finally {
      running.current = undefined;
      setProgress(undefined);
    }
  };

  return {
    start,
    render: (document: OutputDocument | undefined) =>
      document === undefined ? null : (
        <Suspense fallback={null}>
          <PrintDialog
            isOpen={open}
            pageCount={document.pages.length}
            currentPage={document.viewer.getState().currentPage}
            labels={document.labels}
            permissions={encryption?.permissions}
            onPrint={(choice) => void print(choice, document.pages)}
            onClose={() => setOpen(false)}
          />
          <PrintProgressDialog
            progress={progress}
            notice={lowered ? message("print.quality.lowered") : undefined}
            onCancel={() => running.current?.abort()}
          />
        </Suspense>
      ),
  };
}

/**
 * Document properties: the facts read when the dialog opens, once per document; the dialog itself
 * reads what its form edits (properties-editing.tsx).
 */
function useProperties(engine: Engine, platform: Platform, toasts: ToastQueue) {
  const [open, setOpen] = useState(false);
  const [state, setState] = useState<PropertiesState>({ status: "loading" });
  const start = () => {
    setState({ status: "loading" });
    setOpen(true);
    void engine.properties().then(
      (properties) => setState({ status: "ready", properties }),
      (error: unknown) => {
        setState({ status: "failed" });
        reportError(error);
      },
    );
  };
  const copy = (text: string) =>
    void platform.copyText(text).then(() => toasts.add({ message: message("properties.copied") }), reportError);
  return {
    start,
    render: (document: OutputDocument | undefined) =>
      document === undefined ? null : (
        <Suspense fallback={null}>
          <LazyEditablePropertiesDialog
            isOpen={open}
            fileName={document.fileName}
            state={state}
            context={{ pages: document.pages, labels: document.labels }}
            onCopy={copy}
            onClose={() => setOpen(false)}
            engine={engine}
          />
        </Suspense>
      ),
  };
}

/** Download a copy: the bytes as opened, saved under a safe name, then a toast naming it. */
function download(engine: Engine, platform: Platform, toasts: ToastQueue, onProblem: (p: Problem) => void) {
  void (async () => {
    try {
      const { name, bytes } = await engine.originalBytes();
      const result = await platform.saveAs(bytes, downloadName(name));
      if (result.saved) toasts.add({ message: message("download.done", { name: result.name }) });
    } catch (error) {
      onProblem("download");
      reportError(error);
    }
  })();
}

/** The dialog that says printing or a download failed, with Close. */
function ProblemDialog({ problem, onClose }: { problem: Problem | undefined; onClose: () => void }): ReactNode {
  return (
    <Dialog
      isOpen={problem !== undefined}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      role="alertdialog"
      title={message(problem === "download" ? "download.failed" : "print.failed")}
      closeLabel={message("problem.close")}
      actions={
        <Button variant="primary" autoFocus onPress={onClose}>
          {message("problem.close")}
        </Button>
      }
    />
  );
}

/** Print, Download a copy and Document properties for `document`, the open one. */
export function useOutput(
  engine: Engine,
  platform: Platform,
  toasts: ToastQueue,
  document: OutputDocument | undefined,
): Output {
  const [problem, setProblem] = useState<Problem>();
  const print = usePrint(engine, platform, setProblem);
  const properties = useProperties(engine, platform, toasts);
  return {
    actions: {
      "file.print": print.start,
      "file.save-copy": () => download(engine, platform, toasts, setProblem),
      "file.properties": properties.start,
    },
    dialogs: (
      <>
        {print.render(document)}
        {properties.render(document)}
        <ProblemDialog problem={problem} onClose={() => setProblem(undefined)} />
      </>
    ),
  };
}
