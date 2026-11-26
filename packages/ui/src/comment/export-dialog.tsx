// Export comments (Paper C06): XFDF, FDF or Markdown, which comments, what Markdown includes and
// a preview of it, then Copy or Save as…. Export never changes the PDF. With no comments the Save
// button is off and says why.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Select } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { useOnOpen } from "../organise/on-open.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type CommentExportChoice,
  type CommentExportFormat,
  type CommentExportScope,
  EXPORT_FORMATS,
  EXPORT_SCOPES,
  exportName,
  type MarkdownInclude,
  type ScopeCounts,
  scopeCount,
  startChoice,
  withFormat,
} from "./interchange.js";
import { useLoaded } from "./use-loaded.js";

/** What the Markdown preview shows: its text, and how many items it holds on how many pages. */
export interface CommentExportPreview {
  readonly text: string;
  readonly items: number;
  readonly pages: number;
}

/** Props of `ExportCommentsDialog`. */
export interface ExportCommentsDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when the dialog closes. */
  readonly onClose: () => void;
  /** The open file's name, with `.pdf`. */
  readonly fileName: string;
  /** The format it opens on: Markdown from the palette's Export highlights as Markdown. */
  readonly start: CommentExportFormat;
  /** How many comments each choice of Comments names. */
  readonly counts: ScopeCounts;
  /** Reads the Markdown `choice` would write, for the preview. */
  readonly loadPreview: (choice: CommentExportChoice) => Promise<CommentExportPreview>;
  /** Whether a file is being written. */
  readonly busy: boolean;
  /** Writes the file and offers it for saving. */
  readonly onSave: (choice: CommentExportChoice) => void;
  /** Puts the Markdown on the clipboard. */
  readonly onCopy: (choice: CommentExportChoice) => void;
}

/** The format radios, each with its help line. */
function FormatRadios(props: {
  value: CommentExportFormat;
  onChange: (format: CommentExportFormat) => void;
}): ReactNode {
  return (
    <RadioGroup
      label={laterMessage("commentsExport.format")}
      value={props.value}
      onChange={(value) => props.onChange(value as CommentExportFormat)}
    >
      {EXPORT_FORMATS.map((format) => (
        <Radio key={format} value={format}>
          <span>{laterMessage(`commentsExport.format.${format}`)}</span>
          <span className="paper-interchange-help">{laterMessage(`commentsExport.format.${format}.help`)}</span>
        </Radio>
      ))}
    </RadioGroup>
  );
}

/** Page numbers and headings, Author and date, Replies: what Markdown includes. */
function IncludeChecks(props: { value: MarkdownInclude; onChange: (include: MarkdownInclude) => void }): ReactNode {
  const { value } = props;
  return (
    <fieldset className="paper-interchange-group">
      <legend className="paper-field-label">{laterMessage("commentsExport.include")}</legend>
      {(["headings", "authors", "replies"] as const).map((part) => (
        <Checkbox key={part} isSelected={value[part]} onChange={(on) => props.onChange({ ...value, [part]: on })}>
          {laterMessage(`commentsExport.include.${part}`)}
        </Checkbox>
      ))}
    </fieldset>
  );
}

/** The Markdown as it will be written, in a box the keyboard can scroll, with its count. */
function MarkdownPreview({
  choice,
  props,
}: {
  choice: CommentExportChoice;
  props: ExportCommentsDialogProps;
}): ReactNode {
  const key = JSON.stringify([choice.scope, choice.include]);
  const loaded = useLoaded(props.isOpen && choice.format === "markdown", key, () => props.loadPreview(choice));
  if (loaded.state === "loading")
    return <p className="paper-field-help">{laterMessage("commentsExport.preview.loading")}</p>;
  if (loaded.state === "failed") return <Notice tone="danger" title={laterMessage("commentsExport.preview.failed")} />;
  const { text, items, pages } = loaded.value;
  const count = laterMessage("commentsExport.preview.count", {
    items: laterCountMessage(items, { one: "commentsExport.preview.items.one", many: "commentsExport.preview.items" }),
    pages: laterCountMessage(pages, { one: "commentsExport.preview.pages.one", many: "commentsExport.preview.pages" }),
  });
  return (
    <div className="paper-interchange-preview">
      <span className="paper-field-label" id="export-preview-label">
        {laterMessage("commentsExport.preview")}
      </span>
      {items === 0 ? (
        <p className="paper-field-help">{laterMessage("commentsExport.preview.none")}</p>
      ) : (
        <>
          <p className="paper-field-help">{count}</p>
          {/* A region the keyboard can scroll: it takes the focus. */}
          {/* biome-ignore lint/a11y/noNoninteractiveTabindex: a scrollable region must take the focus */}
          <section aria-labelledby="export-preview-label" tabIndex={0} className="paper-interchange-text">
            <pre>{text}</pre>
          </section>
        </>
      )}
    </div>
  );
}

/** Export comments. */
export function ExportCommentsDialog(props: ExportCommentsDialogProps): ReactNode {
  const [choice, setChoice] = useState<CommentExportChoice>(startChoice(props.start));
  useOnOpen(props.isOpen, () => setChoice(startChoice(props.start)));
  const stem = props.fileName.replace(/\.pdf$/i, "");
  const count = scopeCount(props.counts, choice.scope);
  const none = props.counts.all === 0;
  const markdown = choice.format === "markdown";
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open && !props.busy) props.onClose();
      }}
      title={laterMessage("commentsExport.title")}
      description={laterMessage("commentsExport.subtitle", { name: props.fileName })}
      closeLabel={laterMessage("interchange.close")}
      size="form"
      actions={
        <>
          <span className="paper-interchange-hint">
            {laterMessage("commentsExport.name", { name: exportName(stem, choice.format) })}
          </span>
          <Button onPress={props.onClose} isDisabled={props.busy}>
            {laterMessage("interchange.cancel")}
          </Button>
          {markdown ? (
            <Button isDisabled={count === 0 || props.busy} onPress={() => props.onCopy(choice)}>
              {laterMessage("commentsExport.copy")}
            </Button>
          ) : null}
          <Button variant="primary" isDisabled={none || props.busy} onPress={() => props.onSave(choice)}>
            {props.busy ? laterMessage("commentsExport.saving") : laterMessage("commentsExport.save")}
          </Button>
        </>
      }
    >
      <FormatRadios value={choice.format} onChange={(format) => setChoice((c) => withFormat(c, format))} />
      <Select<CommentExportScope>
        label={laterMessage("commentsExport.comments")}
        value={choice.scope}
        onChange={(scope) => setChoice((c) => ({ ...c, scope }))}
        options={EXPORT_SCOPES.map((scope) => ({
          value: scope,
          label: `${laterMessage(`commentsExport.comments.${scope}`)} (${scopeCount(props.counts, scope)})`,
        }))}
        {...(none ? { errorMessage: laterMessage("interchange.noComments") } : {})}
      />
      {markdown ? (
        <>
          <IncludeChecks value={choice.include} onChange={(include) => setChoice((c) => ({ ...c, include }))} />
          <MarkdownPreview choice={choice} props={props} />
        </>
      ) : null}
    </Dialog>
  );
}
