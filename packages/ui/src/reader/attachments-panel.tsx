// The Attachments rail tab (O14), loaded once a document is open (the ./dialogs entry, #380): the
// files the document carries as a list (`role="listbox"`), each with its size, kind and the badge
// naming its relationship and what it belongs to, then the selected file's details: its
// description, what it belongs to, its relationship with its meaning, its facts (type, size, date,
// checksum) and Download (web) or Save to disk… (desktop). In Organise the header's link becomes
// Add file…, a drop zone sits at the foot, the description and relationship become fields, and
// Delete attachment joins the actions. Lamina PDF never opens an attached file: a program gets
// O14's warning before it is saved. Every change is the app's, through the callbacks.

import { Icon } from "@laminapdf/icons/react";
import { type DragEvent, type KeyboardEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { EmptyState } from "../components/empty-state.js";
import { Select, TextArea } from "../components/field.js";
import { keyTarget } from "../components/list-keys.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type AttachmentData,
  type AttachmentRelationshipData,
  annotationPage,
  attachmentKind,
  attachmentLine,
  attachmentName,
  attachmentOwner,
  attachmentsSummary,
  EDITABLE_RELATIONSHIPS,
  isProgram,
  relationshipBadge,
  relationshipMeaning,
  relationshipName,
} from "./attachments.js";
import { formatBytes } from "./bytes.js";
import { formatPdfDate } from "./properties-form.js";

/** What the details say of a file's bytes once they were read: the checksum's verdict. */
export type ChecksumState = "matches" | "mismatch" | "absent" | "checking";

/** What an Organise edit of the selected file changes. */
export interface AttachmentChange {
  readonly description?: string | null;
  readonly relationship?: AttachmentRelationshipData;
}

/** Props of `AttachmentsPanel`. */
export interface AttachmentsPanelProps {
  readonly files: readonly AttachmentData[];
  /** Organise mode: the files can be added, described and deleted. */
  readonly organise: boolean;
  /** The selected file's ID. */
  readonly selected: string | undefined;
  readonly onSelect: (id: string) => void;
  /** A page's name for people: its label, else its number. */
  readonly pageName: (index: number) => string;
  /** The selected file's checksum verdict, once its bytes were read. */
  readonly checksum?: ChecksumState | undefined;
  /** The selected file is larger than the device can hold: the limit, for the message ("512 MB"). */
  readonly tooLarge?: string | undefined;
  /** The save action's name: "Download" on the web, "Save to disk…" on the desktop. */
  readonly saveLabel: string;
  /** Save the file out, after O14's warning for a program. */
  readonly onSave: (file: AttachmentData) => void;
  /** Switch to Organise (Read mode's link). */
  readonly onEditInOrganise?: (() => void) | undefined;
  /** Pick files to attach (Organise). */
  readonly onAdd?: (() => void) | undefined;
  /** Files dropped on the panel, to attach (Organise). */
  readonly onDrop?: ((files: readonly File[]) => void) | undefined;
  /** Change the selected file (Organise). */
  readonly onChange?: ((file: AttachmentData, change: AttachmentChange) => void) | undefined;
  /** Delete the selected file (Organise). */
  readonly onDelete?: ((file: AttachmentData) => void) | undefined;
  /** Go to a page (the Belongs to link). */
  readonly onGoToPage?: ((index: number) => void) | undefined;
  /** A portfolio's column values for a file, as lines under its row ("Owner: Finance"). */
  readonly columns?: ((id: string) => readonly string[]) | undefined;
  /** The note after a deletion was saved incrementally, with Save an optimised copy…. */
  readonly history?: { readonly onSaveOptimised: () => void } | undefined;
}

/** One row: the file's icon, name, size and kind, and its badges. */
function FileRow({
  file,
  pageName,
  columns,
}: {
  file: AttachmentData;
  pageName: (index: number) => string;
  columns: readonly string[];
}): ReactNode {
  const badge = relationshipBadge(file, pageName);
  const program = isProgram(file);
  return (
    <>
      <Icon name="file" size={20} className="paper-attachment-icon" />
      <span className="paper-attachment-text">
        <span className="paper-attachment-name">{attachmentName(file)}</span>
        <span className="paper-attachment-line">{attachmentLine(file)}</span>
        {columns.map((line) => (
          <span key={line} className="paper-attachment-line">
            {line}
          </span>
        ))}
        {badge === undefined && !program ? null : (
          <span className="paper-attachment-badges">
            {program ? (
              <Badge tone="warning" icon="warning">
                {laterMessage("attachments.program")}
              </Badge>
            ) : null}
            {badge === undefined ? null : <Badge>{badge}</Badge>}
          </span>
        )}
      </span>
    </>
  );
}

/** The list's rows: arrow keys, Home and End move the selection, which follows focus. */
function FileList({ props }: { props: AttachmentsPanelProps }): ReactNode {
  const { files, selected, onSelect, pageName } = props;
  const list = useRef<HTMLDivElement>(null);
  const index = Math.max(
    0,
    files.findIndex((f) => f.id === selected),
  );
  const move = (event: KeyboardEvent) => {
    const to = keyTarget(event.key, index, files.length);
    const file = to === undefined ? undefined : files[to];
    if (file === undefined) return;
    event.preventDefault();
    onSelect(file.id);
    list.current?.querySelector<HTMLElement>(`[data-id="${CSS.escape(file.id)}"]`)?.focus();
  };
  return (
    <div ref={list} className="paper-attachments-list" role="listbox" aria-label={laterMessage("attachments.list")}>
      {files.map((file, i) => (
        <div
          key={file.id}
          data-id={file.id}
          role="option"
          aria-selected={file.id === selected}
          tabIndex={i === index ? 0 : -1}
          className="paper-attachment"
          onClick={() => onSelect(file.id)}
          onKeyDown={move}
        >
          <FileRow file={file} pageName={pageName} columns={props.columns?.(file.id) ?? []} />
        </div>
      ))}
    </div>
  );
}

/** The compact drop zone of §7.21 (Organise): it duplicates Add file…, never the only way in. */
function DropZone({ onDrop }: { onDrop: (files: readonly File[]) => void }): ReactNode {
  const [over, setOver] = useState(false);
  const takes = (event: DragEvent) => event.dataTransfer.types.includes("Files");
  return (
    <section
      className="paper-attachments-drop"
      data-over={over ? "" : undefined}
      aria-label={laterMessage("attachments.drop.title")}
      onDragOver={(event) => {
        if (!takes(event)) return;
        event.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(event) => {
        if (!takes(event)) return;
        event.preventDefault();
        setOver(false);
        onDrop([...event.dataTransfer.files]);
      }}
    >
      <Icon name="paperclip" size={16} />
      <span>
        <span className="paper-attachments-drop-title">{laterMessage("attachments.drop.title")}</span>
        <span className="paper-attachments-drop-note">{laterMessage("attachments.drop.note")}</span>
      </span>
    </section>
  );
}

/** The details' subtitle: what the file is associated with, or the comment that holds it. */
function subtitleOf(file: AttachmentData, pageName: (index: number) => string): string {
  const owner = attachmentOwner(file);
  if (owner?.kind === "page") return laterMessage("attachments.subtitle.page", { page: pageName(owner.page) });
  if (owner?.kind === "document") return laterMessage("attachments.subtitle.document");
  const heldOn = annotationPage(file);
  return heldOn === undefined
    ? laterMessage("attachments.subtitle")
    : laterMessage("attachments.subtitle.annotation", { page: pageName(heldOn) });
}

/** What the file belongs to, and the page that goes with it (for the link to it). */
function belongsOf(
  file: AttachmentData,
  pageName: (index: number) => string,
): { readonly text: string; readonly page: number | undefined } {
  const owner = attachmentOwner(file);
  if (owner?.kind === "document") return { text: laterMessage("attachments.belongsTo.document"), page: undefined };
  if (owner?.kind === "page") {
    return { text: laterMessage("attachments.belongsTo.page", { page: pageName(owner.page) }), page: owner.page };
  }
  const page = owner?.page ?? annotationPage(file);
  return page === undefined
    ? { text: laterMessage("attachments.belongsTo.none"), page: undefined }
    : { text: laterMessage("attachments.belongsTo.annotation", { page: pageName(page) }), page };
}

/** The description: text in Read, a field in Organise, committed when it loses focus. */
function DescriptionField({ file, props }: { file: AttachmentData; props: AttachmentsPanelProps }): ReactNode {
  const [description, setDescription] = useState(file.description ?? "");
  // The description changed by undo or redo: the field shows it.
  useEffect(() => setDescription(file.description ?? ""), [file.description]);
  if (!props.organise || props.onChange === undefined) {
    return (
      <dl className="paper-attachment-facts">
        <dt>{laterMessage("attachments.description")}</dt>
        <dd>{file.description ?? laterMessage("attachments.description.none")}</dd>
      </dl>
    );
  }
  const commit = () => {
    const next = description.trim() === "" ? null : description;
    if (next !== (file.description ?? null)) props.onChange?.(file, { description: next });
  };
  return (
    <TextArea
      label={laterMessage("attachments.description")}
      value={description}
      onChange={setDescription}
      onBlur={commit}
    />
  );
}

/** The relationship, with its one-line meaning: text in Read, a select in Organise. */
function RelationshipField({ file, props }: { file: AttachmentData; props: AttachmentsPanelProps }): ReactNode {
  const relationship = file.relationship ?? "unspecified";
  const meaning = relationshipMeaning(relationship, attachmentOwner(file));
  if (!props.organise || props.onChange === undefined) {
    return (
      <dl className="paper-attachment-facts">
        <dt>{laterMessage("attachments.relationship")}</dt>
        <dd>
          {relationshipName(relationship)}
          <span className="paper-attachment-meaning">{meaning}</span>
        </dd>
      </dl>
    );
  }
  return (
    <Select
      label={laterMessage("attachments.relationship")}
      description={meaning}
      value={EDITABLE_RELATIONSHIPS.includes(relationship) ? relationship : "unspecified"}
      options={EDITABLE_RELATIONSHIPS.map((r) => ({ value: r, label: relationshipName(r) }))}
      onChange={(value) => props.onChange?.(file, { relationship: value })}
    />
  );
}

/** The checksum's verdict: a badge once the bytes were read. */
function ChecksumFact({ state }: { state: ChecksumState | undefined }): ReactNode {
  if (state === "matches") {
    return (
      <Badge tone="positive" icon="check">
        {laterMessage("attachments.checksum.matches")}
      </Badge>
    );
  }
  if (state === "mismatch") {
    return (
      <Badge tone="warning" icon="warning">
        {laterMessage("attachments.checksum.mismatch")}
      </Badge>
    );
  }
  return laterMessage(state === "checking" ? "attachments.checksum.checking" : "attachments.checksum.absent");
}

/** The facts: type, size with exact bytes, modification date and checksum. */
function Facts({ file, checksum }: { file: AttachmentData; checksum: ChecksumState | undefined }): ReactNode {
  return (
    <dl className="paper-attachment-facts" aria-label={laterMessage("attachments.facts")}>
      <dt>{laterMessage("attachments.type")}</dt>
      <dd>{file.mime === undefined ? attachmentKind(file) : `${attachmentKind(file)} (${file.mime})`}</dd>
      {file.size === undefined ? null : (
        <>
          <dt>{laterMessage("attachments.size")}</dt>
          <dd>
            {laterMessage("attachments.size.exact", {
              size: formatBytes(file.size),
              bytes: file.size.toLocaleString(),
            })}
          </dd>
        </>
      )}
      {file.modified === undefined ? null : (
        <>
          <dt>{laterMessage("attachments.modified")}</dt>
          <dd>{formatPdfDate(file.modified)}</dd>
        </>
      )}
      <dt>{laterMessage("attachments.checksum")}</dt>
      <dd>
        <ChecksumFact state={checksum} />
      </dd>
    </dl>
  );
}

/** Save out, and Delete attachment (Organise) or Edit in Organise (Read). */
function Actions({ file, props }: { file: AttachmentData; props: AttachmentsPanelProps }): ReactNode {
  return (
    <div className="paper-attachment-actions">
      <Button
        variant="primary"
        size={32}
        icon="download"
        isDisabled={!file.embedded || props.tooLarge !== undefined}
        onPress={() => props.onSave(file)}
      >
        {props.saveLabel}
      </Button>
      {props.organise && props.onDelete !== undefined ? (
        <Button variant="danger" size={32} icon="delete" onPress={() => props.onDelete?.(file)}>
          {laterMessage("attachments.delete")}
        </Button>
      ) : props.onEditInOrganise === undefined ? null : (
        <Button variant="ghost" size={32} onPress={props.onEditInOrganise}>
          {laterMessage("attachments.editInOrganise")}
        </Button>
      )}
    </div>
  );
}

/** The selected file's details: the inspector of O14, under the list in the rail. */
function Details({ file, props }: { file: AttachmentData; props: AttachmentsPanelProps }): ReactNode {
  const titleId = useId();
  const belongs = belongsOf(file, props.pageName);
  const go = props.onGoToPage;
  const page = belongs.page;
  return (
    <section className="paper-attachment-details" aria-labelledby={titleId}>
      <header>
        <h3 id={titleId} className="paper-attachment-details-title">
          {attachmentName(file)}
        </h3>
        <p className="paper-attachment-details-subtitle">{subtitleOf(file, props.pageName)}</p>
      </header>
      <DescriptionField file={file} props={props} />
      <dl className="paper-attachment-facts">
        <dt>{laterMessage("attachments.belongsTo")}</dt>
        <dd>
          {page === undefined || go === undefined ? (
            belongs.text
          ) : (
            <Button variant="ghost" size={26} onPress={() => go(page)}>
              {belongs.text}
            </Button>
          )}
        </dd>
      </dl>
      <RelationshipField file={file} props={props} />
      <Facts file={file} checksum={props.checksum} />
      {props.tooLarge === undefined ? null : (
        <p className="paper-attachment-warning" role="status">
          {laterMessage("attachments.tooLarge", { limit: props.tooLarge })}
        </p>
      )}
      {file.embedded ? null : <p className="paper-attachment-warning">{laterMessage("attachments.notEmbedded")}</p>}
      <Actions file={file} props={props} />
    </section>
  );
}

/** The files the document carries, and the selected one's details. */
export function AttachmentsPanel(props: AttachmentsPanelProps): ReactNode {
  const { files, organise } = props;
  const file = files.find((f) => f.id === props.selected) ?? files[0];
  return (
    <section className="paper-attachments" aria-label={laterMessage("attachments.title")}>
      <header className="paper-attachments-header">
        <span className="paper-attachments-summary">{attachmentsSummary(files)}</span>
        {organise && props.onAdd !== undefined ? (
          <Button variant="ghost" size={26} icon="plus" onPress={props.onAdd}>
            {laterMessage("attachments.add")}
          </Button>
        ) : props.onEditInOrganise === undefined ? null : (
          <Button variant="ghost" size={26} onPress={props.onEditInOrganise}>
            {laterMessage("attachments.editInOrganise")}
          </Button>
        )}
      </header>
      {files.length === 0 ? (
        <EmptyState title={laterMessage("attachments.empty.title")} headingLevel={3} icon="paperclip">
          {laterMessage("attachments.empty.body")}
        </EmptyState>
      ) : (
        <FileList props={props} />
      )}
      {file === undefined ? null : <Details key={file.id} file={file} props={props} />}
      {props.history === undefined ? null : (
        <div className="paper-attachments-history" role="status">
          <p>{laterMessage("attachments.history")}</p>
          <Button size={26} onPress={props.history.onSaveOptimised}>
            {laterMessage("attachments.saveOptimised")}
          </Button>
        </div>
      )}
      {organise && props.onDrop !== undefined ? (
        <DropZone onDrop={props.onDrop} />
      ) : (
        <p className="paper-attachments-note">{laterMessage("attachments.note")}</p>
      )}
    </section>
  );
}

/** Props of `ProgramWarningDialog`. */
export interface ProgramWarningDialogProps {
  /** The program about to be saved; undefined closes the dialog. */
  readonly name: string | undefined;
  /** `true` to save it anyway. */
  readonly onAnswer: (save: boolean) => void;
}

/** O14's warning before a program attached to a document is saved: Cancel / Save anyway. */
export function ProgramWarningDialog({ name, onAnswer }: ProgramWarningDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={name !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={laterMessage("attachments.program.title")}
      description={laterMessage("attachments.program.body")}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {laterMessage("attachments.program.cancel")}
          </Button>
          <Button variant="danger-filled" onPress={() => onAnswer(true)}>
            {laterMessage("attachments.program.save")}
          </Button>
        </>
      }
    />
  );
}
