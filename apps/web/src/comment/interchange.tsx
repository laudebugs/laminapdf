// The dialogs of Export, Import, Flatten and the Comment summary (Paper C06, C07), over the
// interchange controller: each draws only while the controller says it is the one open.

import { useStoreState } from "@laminapdf/ui";
import {
  CommentSummaryDialog,
  ExportCommentsDialog,
  FlattenCommentsDialog,
  ImportCommentsDialog,
} from "@laminapdf/ui/editing";
import type { ReactNode } from "react";
import type { InterchangeController } from "./interchange.ts";

/** Props of `InterchangeDialogs`. */
export interface InterchangeDialogsProps {
  readonly controller: InterchangeController;
  /** The open file's name, with `.pdf`. */
  readonly fileName: string;
}

/** Export comments. */
function ExportDialog({ controller, fileName }: InterchangeDialogsProps): ReactNode {
  const state = useStoreState(controller, (s) => s);
  return (
    <ExportCommentsDialog
      isOpen={state.dialog === "export"}
      onClose={() => controller.close()}
      fileName={fileName}
      start={state.start}
      counts={controller.exportCounts}
      loadPreview={(choice) => controller.preview(choice)}
      busy={state.busy}
      onSave={(choice) => void controller.save(choice)}
      onCopy={(choice) => void controller.copy(choice)}
    />
  );
}

/** Import comments. */
function ImportDialog({ controller }: InterchangeDialogsProps): ReactNode {
  const state = useStoreState(controller, (s) => s);
  return (
    <ImportCommentsDialog
      isOpen={state.dialog === "import"}
      onClose={() => controller.close()}
      file={state.file}
      load={(mode) => controller.loadImport(mode)}
      busy={state.busy}
      onChoose={() => void controller.chooseImport()}
      onImport={(mode) => void controller.doImport(mode)}
    />
  );
}

/** Flatten comments. */
function FlattenDialog({ controller }: InterchangeDialogsProps): ReactNode {
  const state = useStoreState(controller, (s) => s);
  const { counts, stays } = controller.flatten;
  return (
    <FlattenCommentsDialog
      isOpen={state.dialog === "flatten"}
      onClose={() => controller.close()}
      counts={counts}
      stays={stays}
      signatures={state.signatures}
      busy={state.busy}
      onFlatten={(answer, action) => void controller.doFlatten(answer, action)}
    />
  );
}

/** Comment summary. */
function SummaryDialog({ controller, fileName }: InterchangeDialogsProps): ReactNode {
  const state = useStoreState(controller, (s) => s);
  return (
    <CommentSummaryDialog
      isOpen={state.dialog === "summary"}
      onClose={() => controller.close()}
      fileName={fileName}
      counts={controller.summaryCounts}
      run={state.run}
      printFirst={state.printFirst}
      // The press that asks is the gesture a new tab may open in: the controller opens it now.
      onCreate={(choice) => void controller.makeSummary(choice, false)}
      onPrint={(choice) => void controller.makeSummary(choice, true)}
      onCancel={() => controller.cancelSummary()}
    />
  );
}

/** The four dialogs, each open only while the controller names it. */
export function InterchangeDialogs(props: InterchangeDialogsProps): ReactNode {
  return (
    <>
      <ExportDialog {...props} />
      <ImportDialog {...props} />
      <FlattenDialog {...props} />
      <SummaryDialog {...props} />
    </>
  );
}
