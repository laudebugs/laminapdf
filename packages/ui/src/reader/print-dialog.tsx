// Print (R09): which pages, the scale, the quality and whether comments print, then the browser's
// print window. A range names pages by number or label; a range naming no page shows the error
// under the field, keeps Print enabled and puts focus in the field. When the author restricted
// printing (the `/P` print bits, ISO 32000-2 §7.6.4.2) the dialog says so and Print becomes Print
// anyway…, which asks once more in an alertdialog with Cancel focused: the restriction is a
// request, not a lock. When only high quality is restricted, Quality is fixed at Standard. Then a
// progress dialog counts the pages as the worker draws them, and Cancel stops it.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Select, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { ProgressBar } from "../components/progress.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import { parsePrintPages } from "./print-pages.js";

/** What the user chose to print. */
export interface PrintChoice {
  /** 0-based page indices, in order. */
  readonly pages: readonly number[];
  readonly scale: "fit" | "actual";
  /** 150 (standard) or 300 (high). */
  readonly dpi: 150 | 300;
  /** Print annotations as their print flags say. */
  readonly annotations: boolean;
}

/** What the author permits about printing; leave it out for a file that restricts nothing. */
export interface PrintPermissions {
  readonly print: boolean;
  readonly printHighQuality: boolean;
}

/** Props of `PrintDialog`. */
export interface PrintDialogProps {
  isOpen: boolean;
  pageCount: number;
  /** The page in view, 0-based. */
  currentPage: number;
  /** Each page's label, when the document labels its pages. */
  labels?: readonly string[] | undefined;
  permissions?: PrintPermissions | undefined;
  /** Print… (or Print anyway, confirmed) with a valid range: start preparing, and close. */
  onPrint: (choice: PrintChoice) => void;
  /** Cancel, Close, Escape or the scrim. */
  onClose: () => void;
}

type PagesMode = "all" | "current" | "range";

/** The 0-based pages `mode` names, or the range's error. */
function chosenPages(
  mode: PagesMode,
  range: string,
  props: Pick<PrintDialogProps, "pageCount" | "currentPage" | "labels">,
): { pages: readonly number[] } | { error: string } {
  if (mode === "all") return { pages: Array.from({ length: props.pageCount }, (_, i) => i) };
  if (mode === "current") return { pages: [props.currentPage] };
  const result = parsePrintPages(range, props.pageCount, props.labels);
  return result.ok ? { pages: result.pages } : { error: result.error };
}

function currentLabel(props: Pick<PrintDialogProps, "currentPage" | "labels">): string {
  return props.labels?.[props.currentPage] ?? String(props.currentPage + 1);
}

/** The dialog's choices, and what changes them. */
interface PrintForm {
  readonly mode: PagesMode;
  readonly range: string;
  readonly error: string | undefined;
  readonly scale: PrintChoice["scale"];
  readonly quality: "standard" | "high";
  readonly annotations: boolean;
  setMode(mode: PagesMode): void;
  setRange(range: string): void;
  setScale(scale: PrintChoice["scale"]): void;
  setQuality(quality: "standard" | "high"): void;
  setAnnotations(on: boolean): void;
}

/**
 * The choices, kept while the dialog is open, and `submit`: the choice it makes, or the range's
 * error (shown, and the field focused). Each opening starts from every page with no error.
 */
function usePrintForm(
  props: PrintDialogProps,
  standardOnly: boolean,
  focusRange: () => void,
): PrintForm & { submit(): PrintChoice | undefined } {
  const [mode, setModeState] = useState<PagesMode>("all");
  const [range, setRangeState] = useState("");
  const [error, setError] = useState<string | undefined>();
  const [scale, setScale] = useState<PrintChoice["scale"]>("fit");
  const [quality, setQuality] = useState<"standard" | "high">("standard");
  const [annotations, setAnnotations] = useState(true);
  useEffect(() => {
    if (props.isOpen) {
      setModeState("all");
      setError(undefined);
    }
  }, [props.isOpen]);
  return {
    mode,
    range,
    error,
    scale,
    quality,
    annotations,
    setMode: (next) => {
      setModeState(next);
      setError(undefined);
    },
    setRange: (next) => {
      setRangeState(next);
      setModeState("range");
      setError(undefined);
    },
    setScale,
    setQuality,
    setAnnotations,
    submit: () => {
      const chosen = chosenPages(mode, range, props);
      if ("error" in chosen) {
        setError(chosen.error);
        focusRange();
        return undefined;
      }
      return { pages: chosen.pages, scale, dpi: quality === "high" && !standardOnly ? 300 : 150, annotations };
    },
  };
}

/** The pages, scale, quality and comments fields. */
function PrintFields({
  form,
  props,
  standardOnly,
}: {
  form: PrintForm;
  props: PrintDialogProps;
  standardOnly: boolean;
}): ReactNode {
  return (
    <>
      <RadioGroup label={message("print.pages")} value={form.mode} onChange={(v) => form.setMode(v as PagesMode)}>
        <Radio value="all">{message("print.pages.all", { total: props.pageCount })}</Radio>
        <Radio value="current">{message("print.pages.current", { page: currentLabel(props) })}</Radio>
        <Radio value="range">{message("print.pages.range")}</Radio>
      </RadioGroup>
      <div className="paper-print-range">
        <TextInput
          label={message("print.pages.range")}
          placeholder={message("print.pages.rangePlaceholder")}
          description={message("print.pages.rangeHelp")}
          value={form.range}
          onChange={form.setRange}
          onFocus={() => form.setMode("range")}
          {...(form.error === undefined ? {} : { errorMessage: form.error })}
        />
      </div>
      <Select
        label={message("print.scale")}
        value={form.scale}
        onChange={form.setScale}
        options={[
          { value: "fit", label: message("print.scale.fit") },
          { value: "actual", label: message("print.scale.actual") },
        ]}
      />
      <Select
        label={message("print.quality")}
        value={standardOnly ? "standard" : form.quality}
        onChange={form.setQuality}
        isDisabled={standardOnly}
        description={message(standardOnly ? "print.quality.standardOnly" : "print.quality.help")}
        options={[
          { value: "standard", label: message("print.quality.standard") },
          { value: "high", label: message("print.quality.high") },
        ]}
      />
      <Checkbox isSelected={form.annotations} onChange={form.setAnnotations}>
        {message("print.annotations")}
      </Checkbox>
    </>
  );
}

/** The alertdialog before printing a file whose author restricted printing; Cancel has the focus. */
function RestrictedConfirm({
  choice,
  onCancel,
  onPrint,
}: {
  choice: PrintChoice | undefined;
  onCancel: () => void;
  onPrint: (choice: PrintChoice) => void;
}): ReactNode {
  return (
    <Dialog
      isOpen={choice !== undefined}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      role="alertdialog"
      title={message("print.restricted.title")}
      description={message("print.restricted.description")}
      closeLabel={message("print.close")}
      actions={
        <>
          <Button autoFocus onPress={onCancel}>
            {message("print.cancel")}
          </Button>
          <Button
            variant="primary"
            onPress={() => {
              if (choice !== undefined) onPrint(choice);
            }}
          >
            {message("print.restricted.confirm")}
          </Button>
        </>
      }
    />
  );
}

/** Asks which pages to print, and how. */
export function PrintDialog(props: PrintDialogProps): ReactNode {
  const { isOpen, permissions, onPrint, onClose } = props;
  const restricted = permissions !== undefined && !permissions.print;
  const standardOnly = permissions?.print === true && !permissions.printHighQuality;
  const fields = useRef<HTMLFormElement>(null);
  const form = usePrintForm(props, standardOnly, () =>
    fields.current?.querySelector<HTMLInputElement>(".paper-print-range input")?.focus(),
  );
  const [confirming, setConfirming] = useState<PrintChoice | undefined>();
  useEffect(() => {
    if (isOpen) setConfirming(undefined);
  }, [isOpen]);
  const print = () => {
    const choice = form.submit();
    if (choice === undefined) return;
    if (restricted) setConfirming(choice);
    else onPrint(choice);
  };
  return (
    <>
      <Dialog
        isOpen={isOpen && confirming === undefined}
        onOpenChange={(open) => {
          if (!open) onClose();
        }}
        size="form"
        title={message("print.title")}
        closeLabel={message("print.close")}
        actions={
          <>
            <p className="paper-print-next">{message("print.next")}</p>
            <Button onPress={onClose}>{message("print.cancel")}</Button>
            <Button variant="primary" onPress={print}>
              {message(restricted ? "print.printAnyway" : "print.print")}
            </Button>
          </>
        }
      >
        <form
          className="paper-print-fields"
          ref={fields}
          onSubmit={(event) => {
            event.preventDefault();
            print();
          }}
        >
          {restricted ? <Notice tone="warning" title={message("print.restricted")} /> : null}
          <PrintFields form={form} props={props} standardOnly={standardOnly} />
          {/* Enter in a field submits the form, which prints. */}
          <button type="submit" hidden />
        </form>
      </Dialog>
      <RestrictedConfirm
        choice={isOpen ? confirming : undefined}
        onCancel={() => setConfirming(undefined)}
        onPrint={onPrint}
      />
    </>
  );
}

/** Props of `PrintProgressDialog`. */
export interface PrintProgressDialogProps {
  /** Pages ready and the total, while preparing; the dialog is closed without it. */
  progress: { readonly ready: number; readonly total: number } | undefined;
  /** Cancel, Escape: stop preparing. */
  onCancel: () => void;
}

/** Counts the pages drawn for print, with Cancel focused. */
export function PrintProgressDialog({ progress, onCancel }: PrintProgressDialogProps): ReactNode {
  const total = progress?.total ?? 0;
  const ready = progress?.ready ?? 0;
  const percent = total === 0 ? 0 : Math.round((ready / total) * 100);
  return (
    <Dialog
      isOpen={progress !== undefined}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      title={message("print.progress.title")}
      description={message("print.progress.description", { total })}
      isRequired
      onEscape={onCancel}
      actions={
        <Button autoFocus onPress={onCancel}>
          {message("print.cancel")}
        </Button>
      }
    >
      <ProgressBar
        label={message("print.progress.value", { page: Math.min(ready + 1, total), total })}
        value={percent}
      />
    </Dialog>
  );
}
