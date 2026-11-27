// Flatten comments (Paper C07): an alertdialog that asks which comments become part of their pages,
// says what that does to a signed file, and offers to remove the comments from the file's history
// too, which writes a whole new file. A certified file offers a copy; continuing in place needs the
// explicit override. Focus starts on Cancel.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Notice } from "../components/notice.js";
import { laterCountMessage, laterList, laterMessage } from "../later-messages.js";
import { useOnOpen } from "../organise/on-open.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type FlattenAction,
  type FlattenAnswer,
  type FlattenChoice,
  type FlattenCounts,
  type FlattenSignatures,
  flattenAction,
  flattenScopes,
  needsOverride,
  startFlatten,
} from "./interchange.js";

/** Props of `FlattenCommentsDialog`. */
export interface FlattenCommentsDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when the dialog closes. */
  readonly onClose: () => void;
  /** How many comments each scope names. */
  readonly counts: FlattenCounts;
  /** The names of the kinds among `scope`'s comments that can't be flattened (a sound, an attached file). */
  readonly stays: (scope: FlattenChoice) => readonly string[];
  /** How the file's signatures meet flattening. */
  readonly signatures: FlattenSignatures;
  /** Whether the flattening is being done. */
  readonly busy: boolean;
  /** Choices the dialog opens with besides its scope, such as the history option ticked. */
  readonly start?: Partial<FlattenAnswer>;
  /** Flattens as `action` says: here, here with the file rewritten at the next save, or into a copy. */
  readonly onFlatten: (answer: FlattenAnswer, action: FlattenAction) => void;
}

/** The scopes the dialog offers, each with its count. */
function Scopes(props: {
  counts: FlattenCounts;
  value: FlattenChoice;
  onChange: (scope: FlattenChoice) => void;
}): ReactNode {
  return (
    <RadioGroup
      label={laterMessage("flatten.which")}
      value={props.value}
      onChange={(value) => props.onChange(value as FlattenChoice)}
    >
      {flattenScopes(props.counts).map((scope) => (
        <Radio key={scope} value={scope}>
          {laterMessage(`flatten.which.${scope}`, { count: props.counts[scope] })}
        </Radio>
      ))}
    </RadioGroup>
  );
}

/** The notice for the file's signatures: what stays valid, or what this breaks. */
function SignatureNote(props: { signatures: FlattenSignatures; history: boolean }): ReactNode {
  const { signatures, history } = props;
  if (signatures.certified) {
    return (
      <Notice
        tone="danger"
        title={laterMessage(signatures.rightsOnly === true ? "flatten.rights.title" : "flatten.certified.title")}
      >
        {laterMessage(signatures.rightsOnly === true ? "flatten.rights" : "flatten.certified")}
      </Notice>
    );
  }
  if (signatures.approval === 0) return null;
  if (history) {
    return (
      <div role="alert">
        <Notice tone="danger" title={laterMessage("flatten.rewrite.title")}>
          {laterMessage("flatten.rewrite")}
        </Notice>
      </div>
    );
  }
  const { approval } = signatures;
  return (
    <Notice tone="neutral" title={laterMessage("flatten.signed.title")}>
      {approval === 1 ? laterMessage("flatten.signed") : laterMessage("flatten.signed.many", { count: approval })}
    </Notice>
  );
}

/** What can't be flattened in the chosen scope. */
function Leftovers({ stays }: { stays: readonly string[] }): ReactNode {
  if (stays.length === 0) return null;
  const kinds = laterList(stays);
  return (
    <p className="paper-field-help">
      {laterCountMessage(stays.length, { one: "flatten.cannot.one", many: "flatten.cannot" }, { kinds })}
    </p>
  );
}

/** The confirm button's words for what it does. */
function goLabel(action: FlattenAction, count: number): string {
  if (action === "copy") return laterMessage("flatten.go.copy");
  if (action === "rewrite") return laterMessage("flatten.go.rewrite");
  return laterCountMessage(count, { one: "flatten.go.one", many: "flatten.go" });
}

/** The history option, its help and, where an in-place change breaks a signature, the override. */
function HistoryOption(props: {
  answer: FlattenAnswer;
  signatures: FlattenSignatures;
  onChange: (answer: FlattenAnswer) => void;
}): ReactNode {
  const { answer, signatures } = props;
  const signed = signatures.approval > 0 || signatures.certified;
  return (
    <>
      <Checkbox isSelected={answer.history} onChange={(history) => props.onChange({ ...answer, history })}>
        {laterMessage("flatten.history")}
      </Checkbox>
      <p className="paper-field-help">
        {laterMessage(signed ? "flatten.history.help.signed" : "flatten.history.help")}
      </p>
      {needsOverride(signatures, answer.history) ? (
        <Checkbox isSelected={answer.understood} onChange={(understood) => props.onChange({ ...answer, understood })}>
          {laterMessage(
            signatures.rightsOnly === true && !answer.history ? "flatten.understood.rights" : "flatten.understood",
          )}
        </Checkbox>
      ) : null}
    </>
  );
}

/** Flatten comments. */
export function FlattenCommentsDialog(props: FlattenCommentsDialogProps): ReactNode {
  const { counts, signatures } = props;
  const [answer, setAnswer] = useState<FlattenAnswer>({ scope: "all", history: false, understood: false });
  useOnOpen(props.isOpen, () =>
    setAnswer({ scope: startFlatten(counts), history: false, understood: false, ...props.start }),
  );
  const count = counts[answer.scope];
  const { action, enabled } = flattenAction(signatures, answer, count);
  const breaks = action === "rewrite" || (action === "flatten" && signatures.certified);
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open && !props.busy) props.onClose();
      }}
      role="alertdialog"
      title={laterMessage("flatten.title")}
      description={laterMessage("flatten.body")}
      closeLabel={laterMessage("interchange.close")}
      size="form"
      actions={
        <>
          <span className="paper-interchange-hint">
            {laterMessage(answer.history ? "flatten.undo.rewrite" : "flatten.undo")}
          </span>
          <Button autoFocus onPress={props.onClose} isDisabled={props.busy}>
            {laterMessage("interchange.cancel")}
          </Button>
          <Button
            variant={breaks ? "danger-filled" : "primary"}
            isDisabled={!enabled || props.busy}
            onPress={() => props.onFlatten(answer, action)}
          >
            {goLabel(action, count)}
          </Button>
        </>
      }
    >
      <Scopes counts={counts} value={answer.scope} onChange={(scope) => setAnswer((a) => ({ ...a, scope }))} />
      <Leftovers stays={props.stays(answer.scope)} />
      <SignatureNote signatures={signatures} history={answer.history} />
      <HistoryOption answer={answer} signatures={signatures} onChange={setAnswer} />
    </Dialog>
  );
}
