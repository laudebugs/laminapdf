// The Page marks dialog and Manage page marks over the controller's state (Paper A03).

import { useStoreState } from "@laminapdf/ui";
import { ManagePageMarksDialog, PageMarksDialog } from "@laminapdf/ui/editing";
import { type ReactNode, useState } from "react";
import type { PageMarksController } from "./controller.ts";

/** `value` while it is there, and the last one there while it is gone, so a dialog can fade out showing what it showed. */
function useLast<T>(value: T | undefined): T | undefined {
  const [last, setLast] = useState(value);
  if (value !== undefined && value !== last) setLast(value);
  return value ?? last;
}

/** The two dialogs; the dialog reads what its tab needs from the controller. */
export function PageMarksDialogs({ controller }: { controller: PageMarksController }): ReactNode {
  const open = useStoreState(controller, (s) => s.dialog);
  const dialog = useLast(open);
  const opened = useStoreState(controller, (s) => s.manage);
  const manage = useLast(opened);
  const working = useStoreState(controller, (s) => s.working);
  return (
    <>
      {dialog === undefined ? null : (
        <PageMarksDialog
          isOpen={open !== undefined}
          start={dialog.start}
          labels={controller.labels()}
          fileName={controller.fileName()}
          today={dialog.today}
          fonts={controller.fonts()}
          lacks={(fontId, text) => controller.lacks(fontId, text)}
          css={controller.css}
          canHide={dialog.canHide}
          sizeOf={(page) => controller.sizeOf(page)}
          picture={(page) => controller.picture(page)}
          existing={(page) => controller.existing(page)}
          pickImage={() => controller.pickImage()}
          working={working}
          signed={dialog.signed}
          onApply={(choice) => void controller.apply(choice)}
          onClose={() => controller.close()}
          onRemove={dialog.id === undefined ? undefined : () => void controller.remove(dialog.id ?? "")}
        />
      )}
      <ManagePageMarksDialog
        isOpen={opened !== undefined}
        marks={manage ?? []}
        onEdit={(mark) => controller.edit(mark.id)}
        onRemove={(mark) => void controller.remove(mark.id)}
        onClose={() => controller.close()}
      />
    </>
  );
}
