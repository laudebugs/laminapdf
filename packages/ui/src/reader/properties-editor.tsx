// The Description tab of the document properties dialog when the document can be edited (O04):
// title, author, subject, keywords as chips, the creation date, creator and producer, each
// marked when it will be removed (with Keep to take that back), the conformance warning a field
// would cause under it, and the notices about the file's standard and its earlier versions. The
// dialog holds the form; this draws it and reports each change.

import { type ReactNode, useId, useState } from "react";
import { Badge } from "../components/badge.js";
import { Button, IconButton } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { TextArea, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { countMessage, type MessageId, message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type DocumentMetadataData,
  EDITABLE_FIELDS,
  type EditableField,
  type FieldWarning,
  fieldWarnings,
  formatPdfDate,
  hasValue,
  keywordsOf,
  type MetadataForm,
} from "./properties-form.js";

/** Each field's label. */
const FIELD_LABEL: Readonly<Record<EditableField, MessageId>> = {
  title: "properties.title",
  authors: "properties.author",
  subject: "properties.subject",
  keywords: "properties.keywords",
  created: "properties.created",
  creator: "properties.creator",
  producer: "properties.producer",
};

/** `"2B"` as `"PDF/A-2b"`. */
function pdfaLevel(pdfa: string): string {
  const m = /^(\d+)([A-Za-z]*)$/.exec(pdfa);
  return m === null ? `PDF/A-${pdfa}` : `PDF/A-${m[1]}${(m[2] ?? "").toLowerCase()}`;
}

function warningText(warning: FieldWarning, metadata: DocumentMetadataData): string {
  const values = { ...warning.values };
  if (metadata.pdfa !== undefined) values.level = pdfaLevel(metadata.pdfa);
  return message(warning.id, values);
}

/** A field marked for removal: its old value struck through, the badge, and Keep. */
function Removed({ field, value, onKeep }: { field: EditableField; value: string; onKeep: () => void }): ReactNode {
  const label = message(FIELD_LABEL[field]);
  return (
    <div className="paper-field paper-properties-removed">
      <span className="paper-field-label">{label}</span>
      <div className="paper-properties-removed-row">
        <del className="paper-properties-removed-value">{value}</del>
        <Badge tone="danger">{message("properties.edit.willRemove")}</Badge>
        <Button
          variant="ghost"
          size={26}
          aria-label={message("properties.edit.keep.label", { field: label })}
          onPress={onKeep}
        >
          {message("properties.edit.keep")}
        </Button>
      </div>
    </div>
  );
}

/** Keywords as chips, each with its remove button, and an input where Enter or a comma adds one. */
function Keywords({
  keywords,
  onChange,
}: {
  keywords: readonly string[];
  onChange: (keywords: readonly string[]) => void;
}): ReactNode {
  const id = useId();
  const [draft, setDraft] = useState("");
  const add = (text: string) => {
    const added = keywordsOf(text).filter((k) => !keywords.includes(k));
    if (added.length > 0) onChange([...keywords, ...added]);
    setDraft("");
  };
  return (
    <div className="paper-field">
      <label className="paper-field-label" htmlFor={`${id}-input`}>
        {message("properties.keywords")}
      </label>
      <div className="paper-properties-keywords">
        {keywords.length === 0 ? null : (
          // biome-ignore lint/a11y/useSemanticElements: the chips are a group of buttons, not form controls of one value.
          <div
            role="group"
            aria-label={countMessage(keywords.length, {
              one: "properties.edit.keywords.one",
              many: "properties.edit.keywords.many",
            })}
            className="paper-properties-chips"
          >
            {keywords.map((keyword) => (
              <span key={keyword} className="paper-properties-chip">
                {keyword}
                <IconButton
                  icon="close"
                  size={26}
                  label={message("properties.edit.keyword.remove", { keyword })}
                  onPress={() => onChange(keywords.filter((k) => k !== keyword))}
                />
              </span>
            ))}
          </div>
        )}
        <input
          id={`${id}-input`}
          className="paper-input"
          aria-describedby={`${id}-help`}
          value={draft}
          onChange={(e) => {
            const text = e.currentTarget.value;
            if (/[,;]/.test(text)) add(text);
            else setDraft(text);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && draft.trim() !== "") {
              e.preventDefault();
              add(draft);
            } else if (e.key === "Backspace" && draft === "" && keywords.length > 0) {
              onChange(keywords.slice(0, -1));
            }
          }}
          onBlur={() => {
            if (draft.trim() !== "") add(draft);
          }}
        />
      </div>
      <span id={`${id}-help`} className="paper-field-help">
        {message("properties.edit.keywords.help")}
      </span>
    </div>
  );
}

/** The creation date: a native date-and-time field (locale-aware), labelled like the others. */
function Created({ value, onChange }: { value: string; onChange: (value: string) => void }): ReactNode {
  const id = useId();
  return (
    <div className="paper-field">
      <label className="paper-field-label" htmlFor={id}>
        {message("properties.created")}
      </label>
      <input
        id={id}
        className="paper-input"
        type="datetime-local"
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
      />
    </div>
  );
}

/** A field's value as text, for the struck-through line and the removal list. */
function shownValue(metadata: DocumentMetadataData, field: EditableField): string {
  if (field === "authors") return metadata.authors.join("; ");
  if (field === "created") return metadata.created === undefined ? "" : formatPdfDate(metadata.created);
  return metadata[field] ?? "";
}

/** Props of `PropertiesEditor`. */
export interface PropertiesEditorProps {
  metadata: DocumentMetadataData;
  form: MetadataForm;
  onForm: (form: MetadataForm) => void;
  /** Whether Apply removes a value a save keeps in the file's earlier versions. */
  removalStays: boolean;
}

/** The Description tab's form. */
export function PropertiesEditor({ metadata, form, onForm, removalStays }: PropertiesEditorProps): ReactNode {
  const warnings = fieldWarnings(metadata, form);
  const set = (patch: Partial<MetadataForm>) => onForm({ ...form, ...patch });
  const keep = (field: EditableField) => set({ removed: form.removed.filter((f) => f !== field) });
  // A warning isn't an error: the value can still be applied, so the field stays valid and the
  // notice under it says what the value does to the file's standard.
  const warningOf = (field: EditableField): ReactNode => {
    const warning = warnings[field];
    return warning === undefined ? null : <Notice tone="warning" title={warningText(warning, metadata)} />;
  };
  const field = (name: EditableField): ReactNode => {
    if (form.removed.includes(name)) {
      return (
        <div key={name} className="paper-properties-field">
          <Removed field={name} value={shownValue(metadata, name)} onKeep={() => keep(name)} />
          {warningOf(name)}
        </div>
      );
    }
    switch (name) {
      case "keywords":
        return <Keywords key={name} keywords={form.keywords} onChange={(keywords) => set({ keywords })} />;
      case "created":
        return <Created key={name} value={form.created} onChange={(created) => set({ created })} />;
      case "subject":
        return (
          <TextArea
            key={name}
            label={message("properties.subject")}
            value={form.subject}
            onChange={(subject) => set({ subject })}
          />
        );
      default: {
        const help: Partial<Record<EditableField, MessageId>> = {
          authors: "properties.edit.authors.help",
          creator: "properties.edit.creator.help",
          producer: "properties.edit.producer.help",
        };
        const helpId = help[name];
        return (
          <div key={name} className="paper-properties-field">
            <TextInput
              label={message(FIELD_LABEL[name])}
              value={form[name]}
              onChange={(value) => set({ [name]: value })}
              {...(helpId === undefined ? {} : { description: message(helpId) })}
            />
            {warningOf(name)}
          </div>
        );
      }
    }
  };
  const mismatch = metadata.pdfa === undefined ? [] : metadata.disagreements.filter((f) => f !== "modified");
  return (
    <div className="paper-properties-body paper-properties-form">
      {metadata.packet === "malformed" ? (
        <Notice tone="warning" title={message("properties.edit.notice.malformed")}>
          {message("properties.edit.notice.malformed.body")}
        </Notice>
      ) : null}
      {EDITABLE_FIELDS.slice(0, 5).map(field)}
      <div className="paper-field">
        <span className="paper-field-label">{message("properties.modified")}</span>
        <span className="paper-properties-value">
          {metadata.modified === undefined
            ? message("properties.edit.modified.none")
            : formatPdfDate(metadata.modified)}
        </span>
        <span className="paper-field-help">{message("properties.edit.modified.help")}</span>
      </div>
      {EDITABLE_FIELDS.slice(5).map(field)}
      {removalStays ? (
        <Notice tone="warning" title={message("properties.edit.notice.earlier")}>
          {message("properties.edit.notice.earlier.body")}
        </Notice>
      ) : null}
      {metadata.pdfa === undefined ? null : (
        <Notice title={message("properties.edit.notice.pdfa", { level: pdfaLevel(metadata.pdfa) })}>
          {message("properties.edit.notice.pdfa.body")}
          {mismatch.length === 0
            ? null
            : ` ${message("properties.edit.notice.mismatch", {
                fields: mismatch.map((f) => message(FIELD_LABEL[f as EditableField]).toLowerCase()).join(", "),
              })}`}
        </Notice>
      )}
    </div>
  );
}

/** Remove metadata…'s checklist (O04): every field with a value; the checked ones become pending removals. */
export function RemoveMetadataDialog({
  isOpen,
  metadata,
  form,
  onRemove,
  onClose,
}: {
  isOpen: boolean;
  metadata: DocumentMetadataData;
  form: MetadataForm;
  onRemove: (fields: readonly EditableField[]) => void;
  onClose: () => void;
}): ReactNode {
  const [checked, setChecked] = useState<readonly EditableField[]>([]);
  const listed = EDITABLE_FIELDS.filter((f) => hasValue(metadata, f) && !form.removed.includes(f));
  const close = () => {
    setChecked([]);
    onClose();
  };
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) close();
      }}
      title={message("properties.edit.remove.title")}
      description={message("properties.edit.remove.body")}
      closeLabel={message("properties.edit.cancel")}
      actions={
        <>
          <Button onPress={close}>{message("properties.edit.cancel")}</Button>
          <Button
            variant="danger"
            isDisabled={checked.length === 0}
            onPress={() => {
              onRemove(checked);
              close();
            }}
          >
            {message("properties.edit.remove.confirm")}
          </Button>
        </>
      }
    >
      <div className="paper-properties-checklist">
        {listed.map((f) => (
          <Checkbox
            key={f}
            isSelected={checked.includes(f)}
            onChange={(on) => setChecked(on ? [...checked, f] : checked.filter((c) => c !== f))}
          >
            <span className="paper-properties-checklist-label">{message(FIELD_LABEL[f])}</span>
            <span className="paper-properties-checklist-value">{shownValue(metadata, f)}</span>
          </Checkbox>
        ))}
      </div>
    </Dialog>
  );
}
