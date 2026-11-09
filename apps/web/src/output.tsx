// The reader's output (R08, R09): Print… (`file.print`, Ctrl+P), Download a copy (`file.save-copy`)
// and Document properties (`file.properties`), their dialogs, and the toast a download ends with.
// Printing draws each page in the document worker and hands them to the platform one at a time,
// which prints them from a hidden frame; Cancel stops it between pages. Download saves the bytes as
// opened. Nothing is sent anywhere: both are local (ADR 0011).

import {
  Button,
  Dialog,
  message,
  type Platform,
  type PrintChoice,
  PrintDialog,
  PrintProgressDialog,
  PropertiesDialog,
  type PropertiesState,
  type ToastQueue,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useRef, useState } from "react";
import { downloadName } from "./download-name.ts";
import type { PageInfo } from "./outline.ts";
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

/** Whether `error` is a stopped print, which needs no message. */
const aborted = (error: unknown) => error instanceof DOMException && error.name === "AbortError";

/** Print… and its progress: the dialog, then the pages drawn one by one, then the print window. */
function usePrint(engine: Engine, platform: Platform, onProblem: (problem: Problem) => void) {
  const [open, setOpen] = useState(false);
  const [permissions, setPermissions] = useState<DocumentProperties["encryption"]>();
  const [progress, setProgress] = useState<{ ready: number; total: number }>();
  const running = useRef<AbortController | undefined>(undefined);

  const start = () => {
    if (running.current !== undefined) return;
    // The author's permissions decide the dialog's notice; a file whose properties can't be read
    // prints as one that restricts nothing.
    void engine.properties().then(
      (properties) => {
        setPermissions(properties.encryption);
        setOpen(true);
      },
      () => {
        setPermissions(undefined);
        setOpen(true);
      },
    );
  };

  const print = async (choice: PrintChoice) => {
    setOpen(false);
    const controller = new AbortController();
    running.current = controller;
    setProgress({ ready: 0, total: choice.pages.length });
    try {
      await platform.print({
        pages: choice.pages,
        scale: choice.scale,
        signal: controller.signal,
        render: (page, signal) => engine.printPage(page, { dpi: choice.dpi, annotations: choice.annotations }, signal),
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
        <>
          <PrintDialog
            isOpen={open}
            pageCount={document.pages.length}
            currentPage={document.viewer.getState().currentPage}
            labels={document.labels}
            permissions={permissions?.permissions}
            onPrint={(choice) => void print(choice)}
            onClose={() => setOpen(false)}
          />
          <PrintProgressDialog progress={progress} onCancel={() => running.current?.abort()} />
        </>
      ),
  };
}

/** Document properties: read when the dialog opens, once per document. */
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
        <PropertiesDialog
          isOpen={open}
          fileName={document.fileName}
          state={state}
          context={{ pages: document.pages, labels: document.labels }}
          onCopy={copy}
          onClose={() => setOpen(false)}
        />
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
      closeLabel={message("print.close")}
      actions={
        <Button variant="primary" autoFocus onPress={onClose}>
          {message("print.close")}
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
