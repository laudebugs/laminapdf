// Fill & Sign's conflict dialog (Paper P01): the first change to a field a signature forbids
// asks first, naming the field. Cancel, which has the focus, puts the field back; Apply anyway
// keeps the change, and saving then asks again before the signature is invalidated.

import type { ReactNode } from "react";
import { laterMessage } from "../later-messages.js";
import { QuestionDialog } from "../overlays/question-dialog.js";

/** Props of `FieldConflictDialog`. */
export interface FieldConflictDialogProps {
  /** The change asked about: the field's name and why it asks; undefined while nothing is asked. */
  readonly question: { readonly field: string; readonly kind: "certified" | "locked" } | undefined;
  /** The answer: `true` for Apply anyway. */
  readonly onAnswer: (yes: boolean) => void;
}

/** The question before a field's first change on a signed file. */
export function FieldConflictDialog({ question, onAnswer }: FieldConflictDialogProps): ReactNode {
  const locked = question?.kind === "locked";
  return (
    <QuestionDialog
      open={question !== undefined}
      title={laterMessage(locked ? "form.conflict.locked" : "form.conflict.certified")}
      description={laterMessage(locked ? "form.conflict.lockedBody" : "form.conflict.certifiedBody", {
        field: question?.field ?? "",
      })}
      no={laterMessage("form.conflict.cancel")}
      yes={laterMessage("form.conflict.apply")}
      yesVariant="danger"
      onAnswer={onAnswer}
    />
  );
}
