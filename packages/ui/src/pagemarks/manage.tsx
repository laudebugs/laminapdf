// Manage page marks (Paper A03): the marks Lamina PDF added to the file, one row each with what it
// says and the pages it is on, to edit or take off. A row's Edit opens the Page marks dialog on
// that mark; Remove takes it off every page as one step the toast can undo.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { EmptyState } from "../components/empty-state.js";
import { Skeleton } from "../components/progress.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** What a mark is: a background is a watermark behind the content. */
export type ManagedKind = "watermark" | "background" | "headerFooter" | "bates";

/** One mark in the list. */
export interface ManagedMark {
  readonly id: string;
  readonly kind: ManagedKind;
  /** What it says: the watermark's text, the header's slots, the Bates number's first and last. */
  readonly summary: string;
  /** The pages it is on, as a person says them ("2–24"). */
  readonly pages: string;
}

/** Props of `ManagePageMarksDialog`. */
export interface ManagePageMarksDialogProps {
  readonly isOpen: boolean;
  /** The marks, or why there are none to show yet. */
  readonly marks: readonly ManagedMark[] | "reading" | "failed";
  /** Opens the mark in the Page marks dialog. */
  readonly onEdit: (mark: ManagedMark) => void;
  /** Takes the mark off every page it is on. */
  readonly onRemove: (mark: ManagedMark) => void;
  /** Done, the close button or Escape. */
  readonly onClose: () => void;
}

/** The list of marks, with an empty state and a note while it is read. */
export function ManagePageMarksDialog({
  isOpen,
  marks,
  onEdit,
  onRemove,
  onClose,
}: ManagePageMarksDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={laterMessage("pagemarks.manage.title")}
      description={laterMessage("pagemarks.manage.description")}
      closeLabel={laterMessage("pagemarks.manage.close")}
      size="form"
      actions={
        <Button variant="primary" onPress={onClose}>
          {laterMessage("pagemarks.manage.done")}
        </Button>
      }
    >
      {marks === "reading" ? (
        <Skeleton blockSize="48px" />
      ) : marks === "failed" ? (
        <p role="alert">{laterMessage("pagemarks.manage.failed")}</p>
      ) : marks.length === 0 ? (
        <EmptyState title={laterMessage("pagemarks.manage.empty")} headingLevel={3}>
          {laterMessage("pagemarks.manage.emptyHelp")}
        </EmptyState>
      ) : (
        <ul className="paper-marks-list" aria-label={laterMessage("pagemarks.manage.list")}>
          {marks.map((mark) => {
            const name = laterMessage(`pagemarks.kind.${mark.kind}`);
            return (
              <li key={mark.id} className="paper-marks-row-item">
                <div className="paper-marks-row-text">
                  <p className="paper-marks-row-kind">{name}</p>
                  <p className="paper-marks-row-summary">{mark.summary}</p>
                  <p className="paper-marks-row-pages">
                    {laterMessage("pagemarks.manage.pages", { pages: mark.pages })}
                  </p>
                </div>
                <Button
                  size={30}
                  aria-label={laterMessage("pagemarks.manage.edit", { kind: name, summary: mark.summary })}
                  onPress={() => onEdit(mark)}
                >
                  {laterMessage("pagemarks.manage.editLabel")}
                </Button>
                <Button
                  size={30}
                  variant="ghost"
                  aria-label={laterMessage("pagemarks.manage.remove", { kind: name, summary: mark.summary })}
                  onPress={() => onRemove(mark)}
                >
                  {laterMessage("pagemarks.manage.removeLabel")}
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </Dialog>
  );
}
