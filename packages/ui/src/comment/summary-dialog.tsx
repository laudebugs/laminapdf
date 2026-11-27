// Comment summary (Paper C07): a new document that lists the comments beside their pages. The
// options, what the summary will list, and, while it is made, an indeterminate bar with Cancel
// (the summary is one call, not a job, so there is no step to count; Cancel drops the result).
// The pages it couldn't draw are named once it is made. It opens as a new, unsaved document.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Select } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { ProgressBar } from "../components/progress.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { useOnOpen } from "../organise/on-open.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type NotDrawn,
  pageList,
  START_SUMMARY,
  SUMMARY_LAYOUTS,
  type SummaryChoice,
  type SummaryComments,
  type SummaryPaper,
} from "./interchange.js";

/** How many comments a choice of Comments lists, and on how many pages. */
export interface SummaryCount {
  readonly comments: number;
  readonly pages: number;
}

/** Where the summary opened, or didn't. */
export type SummaryOpened = "tab" | "window" | "download";

/** What the dialog is doing now. */
export type SummaryRun =
  | { readonly kind: "idle" }
  | { readonly kind: "making"; readonly print: boolean }
  | { readonly kind: "done"; readonly opened: SummaryOpened | "printed"; readonly notDrawn: readonly NotDrawn[] }
  | { readonly kind: "failed"; readonly reason: "glyph" | "limit" | "other" };

/** Props of `CommentSummaryDialog`. */
export interface CommentSummaryDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when the dialog closes. */
  readonly onClose: () => void;
  /** The open file's name, with `.pdf`. */
  readonly fileName: string;
  /** What each choice of Comments lists. */
  readonly counts: Readonly<Record<SummaryComments, SummaryCount>>;
  /** Where making the summary is. */
  readonly run: SummaryRun;
  /** Opened to print: Print… is the primary button, and Create summary the other way out. */
  readonly printFirst?: boolean;
  /** Makes the summary and opens it. */
  readonly onCreate: (choice: SummaryChoice) => void;
  /** Makes the summary and prints it. */
  readonly onPrint: (choice: SummaryChoice) => void;
  /** Drops the summary being made. */
  readonly onCancel: () => void;
}

/** Layout, Comments, Include and Paper size. */
function Options(props: {
  choice: SummaryChoice;
  counts: CommentSummaryDialogProps["counts"];
  disabled: boolean;
  onChange: (choice: SummaryChoice) => void;
}): ReactNode {
  const { choice, counts, disabled } = props;
  const set = (change: Partial<SummaryChoice>) => props.onChange({ ...choice, ...change });
  return (
    <>
      <RadioGroup
        label={laterMessage("summary.layout")}
        value={choice.layout}
        isDisabled={disabled}
        onChange={(layout) => set({ layout: layout as SummaryChoice["layout"] })}
      >
        {SUMMARY_LAYOUTS.map((layout) => (
          <Radio key={layout} value={layout}>
            {laterMessage(`summary.layout.${layout}`)}
          </Radio>
        ))}
      </RadioGroup>
      <RadioGroup
        label={laterMessage("summary.comments")}
        value={choice.comments}
        isDisabled={disabled}
        onChange={(comments) => set({ comments: comments as SummaryComments })}
      >
        {(["all", "shown"] as const).map((comments) => (
          <Radio key={comments} value={comments}>
            {laterMessage(`summary.comments.${comments}`, { count: counts[comments].comments })}
          </Radio>
        ))}
      </RadioGroup>
      <Include choice={choice} disabled={disabled} onChange={set} />
      <Select<SummaryPaper>
        label={laterMessage("summary.paper")}
        value={choice.paper}
        isDisabled={disabled}
        onChange={(paper) => set({ paper })}
        options={(["document", "a4", "letter"] as const).map((paper) => ({
          value: paper,
          label: laterMessage(`summary.paper.${paper}`),
        }))}
      />
    </>
  );
}

/** Replies, Status and Pages without comments. */
function Include(props: {
  choice: SummaryChoice;
  disabled: boolean;
  onChange: (change: Partial<SummaryChoice>) => void;
}): ReactNode {
  const { choice } = props;
  return (
    <fieldset className="paper-interchange-group">
      <legend className="paper-field-label">{laterMessage("summary.include")}</legend>
      {(["replies", "status", "emptyPages"] as const).map((part) => (
        <Checkbox
          key={part}
          isSelected={choice[part]}
          isDisabled={props.disabled}
          onChange={(on) => props.onChange({ [part]: on })}
        >
          {laterMessage(`summary.include.${part}`)}
        </Checkbox>
      ))}
    </fieldset>
  );
}

/** What the summary will list, in a sentence. */
function About({ count }: { count: SummaryCount }): ReactNode {
  const pages = laterCountMessage(count.pages, { one: "summary.about.pages.one", many: "summary.about.pages" });
  return (
    <p className="paper-field-help">
      {laterCountMessage(count.comments, { one: "summary.about.one", many: "summary.about" }, { pages })}
    </p>
  );
}

/** What each way of finishing says. */
const OPENED = {
  tab: "summary.done",
  window: "summary.done.window",
  download: "summary.done.download",
  printed: "summary.done.printed",
} as const;

/** What happened once the summary was made: where it opened and the pages it couldn't draw, or why it failed. */
function Outcome({ run }: { run: SummaryRun }): ReactNode {
  if (run.kind === "failed") {
    return (
      <Notice tone="danger" title={laterMessage("summary.failed.title")}>
        {laterMessage(
          run.reason === "glyph"
            ? "summary.failed.glyph"
            : run.reason === "limit"
              ? "summary.failed.limit"
              : "summary.failed",
        )}
      </Notice>
    );
  }
  if (run.kind !== "done") return null;
  const pages = pageList(run.notDrawn.map((n) => n.page));
  return (
    <>
      <Notice tone="positive" title={laterMessage(OPENED[run.opened])} />
      {run.notDrawn.length === 0 ? null : (
        <Notice tone="warning" title={laterMessage("summary.notDrawn.title")}>
          {laterCountMessage(run.notDrawn.length, { one: "summary.notDrawn.one", many: "summary.notDrawn" }, { pages })}
        </Notice>
      )}
    </>
  );
}

/** The buttons: Cancel (drops the summary being made), Print… and Create summary, or Close once made. */
function Actions(props: {
  run: SummaryRun;
  none: boolean;
  props: CommentSummaryDialogProps;
  choice: SummaryChoice;
}): ReactNode {
  const { run, choice } = props;
  const making = run.kind === "making";
  if (run.kind === "done") {
    return (
      <Button variant="primary" autoFocus onPress={props.props.onClose}>
        {laterMessage("interchange.close")}
      </Button>
    );
  }
  const printFirst = props.props.printFirst === true;
  const print = (
    <Button
      key="print"
      variant={printFirst ? "primary" : "secondary"}
      isDisabled={props.none || making}
      onPress={() => props.props.onPrint(choice)}
    >
      {laterMessage("summary.print")}
    </Button>
  );
  const create = (
    <Button
      key="create"
      variant={printFirst ? "secondary" : "primary"}
      isDisabled={props.none || making}
      onPress={() => props.props.onCreate(choice)}
    >
      {laterMessage("summary.create")}
    </Button>
  );
  // The primary button comes last.
  const [first, second] = printFirst ? [create, print] : [print, create];
  return (
    <>
      <Button onPress={making ? props.props.onCancel : props.props.onClose}>
        {laterMessage("interchange.cancel")}
      </Button>
      {first}
      {second}
    </>
  );
}

/** Comment summary. */
export function CommentSummaryDialog(props: CommentSummaryDialogProps): ReactNode {
  const [choice, setChoice] = useState<SummaryChoice>(START_SUMMARY);
  useOnOpen(props.isOpen, () => setChoice(START_SUMMARY));
  const { run } = props;
  const making = run.kind === "making";
  const count = props.counts[choice.comments];
  const none = count.comments === 0;
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) (making ? props.onCancel : props.onClose)();
      }}
      title={laterMessage("summary.title")}
      description={laterMessage("summary.opens", { name: props.fileName })}
      closeLabel={laterMessage("interchange.close")}
      size="form"
      actions={<Actions run={run} none={none} props={props} choice={choice} />}
    >
      <Options choice={choice} counts={props.counts} disabled={making || run.kind === "done"} onChange={setChoice} />
      {none ? (
        <Notice tone="neutral" title={laterMessage("interchange.noComments.summary")} />
      ) : (
        <About count={count} />
      )}
      {making ? <ProgressBar label={laterMessage(run.print ? "summary.printing" : "summary.making")} /> : null}
      <Outcome run={run} />
    </Dialog>
  );
}
