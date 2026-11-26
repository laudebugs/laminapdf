// The Form data dialogs (the Form data menu): the export dialog, the reset and flatten questions, and the
// error for a file that isn't form data. Each is a controlled dialog; the reader owns what it asks
// and does the work. Reset and flatten are alert dialogs whose focus starts on Cancel.

import { type ReactNode, useEffect, useState } from "react";
import { Button } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { TextInput } from "../components/field.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import { QuestionDialog } from "../overlays/question-dialog.js";

/** A form data file's format. */
export type FormDataFileFormat = "json" | "xfdf" | "fdf";

/** What the export dialog asks for. */
export interface ExportChoice {
  readonly format: FormDataFileFormat;
  /** Only the fields with an answer. */
  readonly filledOnly: boolean;
  /** The file's name, with its extension. */
  readonly name: string;
}

/** Props of `ExportFormDataDialog`. */
export interface ExportFormDataDialogProps {
  readonly open: boolean;
  /** The PDF's name without its extension: the file name starts as this. */
  readonly baseName: string;
  /** How many fields have an answer, and how many there are. */
  readonly counts: { readonly filled: number; readonly total: number };
  /** The browser downloads the file: the help line says so, instead of "choose where to save it". */
  readonly downloads: boolean;
  /** The dialog was dismissed or Cancel was pressed. */
  readonly onCancel: () => void;
  /** Export… was pressed with these choices. */
  readonly onExport: (choice: ExportChoice) => void;
}

const FORMATS: readonly FormDataFileFormat[] = ["json", "xfdf", "fdf"];

/** `name` with the extension of `format`, replacing the one of another format it already has. */
export function nameFor(name: string, format: FormDataFileFormat): string {
  const stem = name.replace(/\.(json|xfdf|fdf)$/i, "");
  return `${stem}.${format}`;
}

/** The export dialog: format, which fields and the file's name. */
export function ExportFormDataDialog(props: ExportFormDataDialogProps): ReactNode {
  const { open, baseName, counts, downloads, onCancel, onExport } = props;
  const [format, setFormat] = useState<FormDataFileFormat>("json");
  const [scope, setScope] = useState<"filled" | "all">("filled");
  const [name, setName] = useState(nameFor(`${baseName}-data`, "json"));
  // Each opening starts from the defaults; the name follows the format until it is typed over.
  useEffect(() => {
    if (!open) return;
    setFormat("json");
    setScope(counts.filled > 0 ? "filled" : "all");
    setName(nameFor(`${baseName}-data`, "json"));
  }, [open, baseName, counts.filled]);
  const chooseFormat = (next: FormDataFileFormat) => {
    setFormat(next);
    setName((was) => nameFor(was, next));
  };
  const clean = name.trim();
  return (
    <Dialog
      isOpen={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onCancel();
      }}
      title={laterMessage("form.export.title")}
      description={laterMessage("form.export.subtitle")}
      closeLabel={laterMessage("form.export.close")}
      size="form"
      actions={
        <>
          <Button onPress={onCancel}>{laterMessage("form.export.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={clean === ""}
            onPress={() => onExport({ format, filledOnly: scope === "filled", name: nameFor(clean, format) })}
          >
            {laterMessage("form.export.go")}
          </Button>
        </>
      }
    >
      <RadioGroup
        label={laterMessage("form.export.format")}
        value={format}
        onChange={(value) => chooseFormat(value as FormDataFileFormat)}
      >
        {FORMATS.map((f) => (
          <Radio key={f} value={f}>
            <strong>{laterMessage(`form.export.${f}`)}</strong> {laterMessage(`form.export.${f}Help`)}
          </Radio>
        ))}
      </RadioGroup>
      <RadioGroup
        label={laterMessage("form.export.fields")}
        value={scope}
        onChange={(value) => setScope(value as "filled" | "all")}
      >
        <Radio value="filled">{laterMessage("form.export.filled", { count: counts.filled })}</Radio>
        <Radio value="all">{laterMessage("form.export.all", { count: counts.total })}</Radio>
      </RadioGroup>
      <TextInput
        label={laterMessage("form.export.name")}
        value={name}
        onChange={setName}
        description={laterMessage(downloads ? "form.export.helpDownload" : "form.export.help")}
      />
    </Dialog>
  );
}

/** Props of `ResetFormDialog`. */
export interface ResetFormDialogProps {
  readonly open: boolean;
  /** The fields reset, and how many a signature locks, which stay as they are. */
  readonly fields: number;
  readonly locked: number;
  /** The answer: `true` for Reset form. */
  readonly onAnswer: (yes: boolean) => void;
}

/** "Reset all 27 fields?" with the locked ones named. */
export function ResetFormDialog({ open, fields, locked, onAnswer }: ResetFormDialogProps): ReactNode {
  return (
    <QuestionDialog
      open={open}
      title={laterCountMessage(fields, { one: "form.reset.title.one", many: "form.reset.title.many" })}
      description={laterMessage("form.reset.body")}
      no={laterMessage("form.reset.cancel")}
      yes={laterMessage("form.reset.confirm")}
      yesVariant="danger-filled"
      onAnswer={onAnswer}
    >
      {locked === 0 ? null : (
        <p>{laterCountMessage(locked, { one: "form.reset.locked.one", many: "form.reset.locked.many" })}</p>
      )}
    </QuestionDialog>
  );
}

/** Props of `FlattenFormDialog`. */
export interface FlattenFormDialogProps {
  readonly open: boolean;
  /** The fields flattened, how many a signature locks (left as they are) and whether a signature field stays. */
  readonly fields: number;
  readonly locked: number;
  readonly hasSignatureField: boolean;
  /**
   * The file's certification says flattening breaks it: the dialog is the conflict confirmation,
   * naming the signer when it is known, with Save a copy… beside Cancel and Flatten anyway.
   */
  readonly certified?: { readonly signer?: string; readonly copyTitle: string } | undefined;
  /** Save a copy… was pressed, which leaves the certified file as it is. */
  readonly onCopy?: () => void;
  /** The answer: `true` for the flatten, `false` for Cancel. */
  readonly onAnswer: (yes: boolean) => void;
}

/** The flatten dialog; with a certification it is also the conflict confirmation. */
export function FlattenFormDialog(props: FlattenFormDialogProps): ReactNode {
  const { open, fields, locked, hasSignatureField, certified, onCopy, onAnswer } = props;
  const warning =
    certified === undefined
      ? undefined
      : certified.signer === undefined
        ? laterMessage("form.flatten.certifiedBare")
        : laterMessage("form.flatten.certified", { signer: certified.signer });
  return (
    <Dialog
      isOpen={open}
      isRequired
      role="alertdialog"
      onOpenChange={(isOpen) => {
        if (!isOpen) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={laterMessage("form.flatten.title")}
      description={laterMessage("form.flatten.body")}
      actions={
        <>
          {certified === undefined || onCopy === undefined ? null : (
            <Button onPress={onCopy}>{certified.copyTitle}</Button>
          )}
          <Button autoFocus onPress={() => onAnswer(false)}>
            {laterMessage("form.flatten.cancel")}
          </Button>
          <Button variant="danger-filled" onPress={() => onAnswer(true)}>
            {certified === undefined
              ? laterCountMessage(fields, { one: "form.flatten.go.one", many: "form.flatten.go.many" })
              : laterMessage("form.flatten.anyway")}
          </Button>
        </>
      }
    >
      {warning === undefined ? null : <p role="alert">{warning}</p>}
      <RadioGroup label={laterMessage("form.flatten.scope")} value="every" isReadOnly>
        <Radio value="every">{laterMessage("form.flatten.every", { count: fields })}</Radio>
        <Radio value="selected" isDisabled>
          {laterMessage("form.flatten.selected")}
        </Radio>
      </RadioGroup>
      <p className="paper-form-dialog-help">{laterMessage("form.flatten.noSelection")}</p>
      {hasSignatureField ? <p>{laterMessage("form.flatten.signature")}</p> : null}
      {locked === 0 ? null : (
        <p>{laterCountMessage(locked, { one: "form.flatten.locked.one", many: "form.flatten.locked.many" })}</p>
      )}
      <p>{laterMessage("form.flatten.undo")}</p>
    </Dialog>
  );
}

/** Props of `ImportErrorDialog`. */
export interface ImportErrorDialogProps {
  /** The file and why it can't be read; undefined while nothing failed. */
  readonly error: { readonly file: string; readonly reason: string } | undefined;
  /** OK or the close button was pressed. */
  readonly onClose: () => void;
}

/** "This file isn’t valid XFDF (line 14: …)": a file that isn't form data, which imports nothing. */
export function ImportErrorDialog({ error, onClose }: ImportErrorDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={error !== undefined}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose();
      }}
      title={laterMessage("form.import.errorTitle", { file: error?.file ?? "" })}
      description={error?.reason ?? ""}
      closeLabel={laterMessage("form.import.errorClose")}
      actions={
        <Button variant="primary" autoFocus onPress={onClose}>
          {laterMessage("form.import.errorOk")}
        </Button>
      }
    />
  );
}
