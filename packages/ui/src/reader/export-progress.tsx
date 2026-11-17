// Export's progress on the web (Paper O16): what is being made, a determinate bar counting pages or
// pictures, where the result goes (a zip, one file or a folder; nothing is uploaded) and Cancel
// export, which keeps focus while it runs. Pages that couldn't be drawn are listed, with Export the
// rest. Screen readers hear every 25 %, as print's progress does, not every page.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { ProgressBar } from "../components/progress.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import type { ExportChoice } from "./export-dialog.js";

/** Where an export's result goes: the zip's name, the one file's name, or the folder's. */
export type ExportDestination = { readonly zip: string } | { readonly file: string } | { readonly folder: string };

/** Props of `ExportProgressDialog`. */
export interface ExportProgressDialogProps {
  isOpen: boolean;
  /** What is being made: `exportProgressTitle` builds it. */
  title: string;
  /** Pages or pictures done, and how many in all. */
  done: number;
  total: number;
  /** What `done` and `total` count. */
  unit: "pages" | "pictures";
  /**
   * Where the result goes, named in the sentence under the bar: a zip of many files, one file, or a
   * folder written into. Left out, the sentence says only that nothing is uploaded.
   */
  destination?: ExportDestination | undefined;
  /** 0-based pages that couldn't be drawn: listed, with Export the rest. */
  failed?: readonly number[] | undefined;
  /** Cancel export, or Escape, while it runs. */
  onCancel: () => void;
  /** Export the rest: the pages that were drawn, without the failed ones. */
  onExportRest?: (() => void) | undefined;
  /** Close, once pages have failed. */
  onClose: () => void;
}

const UPPER: Record<Extract<ExportChoice, { kind: "pages" }>["format"], string> = {
  png: "PNG",
  jpeg: "JPEG",
  tiff: "TIFF",
};

/** The progress dialog's title for `choice`: "Exporting 24 pages as PNG"; `count` is its pages. */
export function exportProgressTitle(choice: ExportChoice, count: number): string {
  switch (choice.kind) {
    case "pages":
      return laterCountMessage(
        count,
        { one: "export.progress.pages.one", many: "export.progress.pages" },
        { format: UPPER[choice.format] },
      );
    case "text":
      return laterMessage("export.progress.text", {
        format: laterMessage(choice.format === "text" ? "export.progress.text.text" : `export.format.${choice.format}`),
      });
    case "pictures":
      return laterCountMessage(count, { one: "export.progress.pictures.one", many: "export.progress.pictures" });
    case "links":
      return laterMessage("export.progress.links");
  }
}

/**
 * The sentence under the bar. It doesn't say how a zip arrives: the browser may have asked where
 * to save it before the work began, or may download it at the end.
 */
function whereText(destination: ExportDestination | undefined, unit: ExportProgressDialogProps["unit"]): string {
  if (destination === undefined) return laterMessage("export.progress.here");
  if ("zip" in destination) return laterMessage(`export.progress.zip.${unit}`, { name: destination.zip });
  if ("file" in destination) return laterMessage("export.progress.file", { name: destination.file });
  return laterMessage(`export.progress.folder.${unit}`, { folder: destination.folder });
}

/** The quarter reached, 0 to 4. */
const quarter = (done: number, total: number) => (total === 0 ? 0 : Math.floor((done / total) * 4));

/** The pages that couldn't be drawn, one line each. */
function FailedPages({ failed }: { failed: readonly number[] }): ReactNode {
  return (
    <Notice tone="danger" title={laterMessage("export.progress.failed")}>
      <span className="paper-export-failed">
        {failed.map((page) => (
          <span key={page}>{laterMessage("export.progress.failedPage", { page: page + 1 })}</span>
        ))}
      </span>
    </Notice>
  );
}

/** The bar, its live quarter and where the result goes. */
function ExportProgressBody(props: ExportProgressDialogProps): ReactNode {
  const { done, total, unit } = props;
  // Down, so it never reads 100 % before the last page is done.
  const percent = total === 0 ? 0 : Math.floor((done / total) * 100);
  const announced = quarter(done, total) * 25;
  return (
    <>
      {/* The bar itself isn't live: its label changes every page. This region speaks each quarter. */}
      <ProgressBar
        label={laterCountMessage(
          total,
          { one: `export.progress.value.${unit}.one`, many: `export.progress.value.${unit}` },
          { done, total },
        )}
        value={percent}
      />
      <p className="paper-visually-hidden" aria-live="polite">
        {announced === 0 ? "" : laterMessage("export.progress.announce", { percent: announced })}
      </p>
      <p className="paper-export-where">{whereText(props.destination, unit)}</p>
    </>
  );
}

/** Counts what an export has made, with Cancel export focused; lists the pages that failed. */
export function ExportProgressDialog(props: ExportProgressDialogProps): ReactNode {
  const failed = props.failed ?? [];
  const running = failed.length === 0;
  const frame = running
    ? ({ isRequired: true, onEscape: props.onCancel } as const)
    : ({ closeLabel: laterMessage("export.close") } as const);
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) (running ? props.onCancel : props.onClose)();
      }}
      title={props.title}
      {...frame}
      actions={
        running ? (
          <Button autoFocus onPress={props.onCancel}>
            {laterMessage("export.progress.cancel")}
          </Button>
        ) : (
          <>
            <Button onPress={props.onClose}>{laterMessage("export.cancel")}</Button>
            {props.onExportRest === undefined ? null : (
              <Button variant="primary" autoFocus onPress={props.onExportRest}>
                {laterMessage("export.progress.rest")}
              </Button>
            )}
          </>
        )
      }
    >
      <ExportProgressBody {...props} />
      {running ? null : <FailedPages failed={failed} />}
    </Dialog>
  );
}
