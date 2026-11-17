/**
 * A yes-or-no question as an alert dialog, which the focus starts on the safe answer of: Escape,
 * closing and the first button all answer no.
 */

import type { ReactNode } from "react";
import { Button, type ButtonVariant } from "../components/button.js";
import { Dialog } from "./dialog.js";

/** Props of `QuestionDialog`. */
export interface QuestionDialogProps {
  /** Whether the question is asked. */
  readonly open: boolean;
  /** The question. */
  readonly title: string;
  /** What answering yes does. */
  readonly description: string;
  /** The safe answer's label, which has the focus. */
  readonly no: string;
  /** The other answer's label. */
  readonly yes: string;
  /** How the yes button looks: `danger-filled` when it can do harm. */
  readonly yesVariant: ButtonVariant;
  /** The answer: `true` for yes. */
  readonly onAnswer: (yes: boolean) => void;
}

/** Asks a yes-or-no question; anything but the yes button answers no. */
export function QuestionDialog(props: QuestionDialogProps): ReactNode {
  const { onAnswer } = props;
  return (
    <Dialog
      isOpen={props.open}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={props.title}
      description={props.description}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {props.no}
          </Button>
          <Button variant={props.yesVariant} onPress={() => onAnswer(true)}>
            {props.yes}
          </Button>
        </>
      }
    />
  );
}
