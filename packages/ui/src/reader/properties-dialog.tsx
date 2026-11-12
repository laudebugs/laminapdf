// The document properties dialog (R08): everything Lamina PDF's own parser knows about the file,
// in four sections (Description, Document, Fonts, Security), at form width. Each value has a copy
// button named by what it copies, shown on hover or focus; Copy all writes every section as plain
// text. When the document can be edited (O04), the Description section becomes a form on its own
// tab, the rest moves to a File tab, and Apply makes the form's changes one edit; nothing reaches
// the file until it is saved.

import { type ReactNode, useState } from "react";
import { Button, IconButton } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { SegmentedTabs, TabPanel } from "../components/tabs.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type DocumentPropertiesData,
  fontStorage,
  type PropertiesContext,
  type PropertySection,
  propertiesText,
  propertySections,
} from "./properties.js";
import { PropertiesEditor, RemoveMetadataDialog } from "./properties-editor.js";
import {
  changeOf,
  type DocumentMetadataData,
  EDITABLE_FIELDS,
  formFrom,
  hasValue,
  type MetadataChange,
  type MetadataForm,
  removalStaysInEarlierVersions,
} from "./properties-form.js";

/** What the dialog shows: the properties once read, or that they are being read or couldn't be. */
export type PropertiesState =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly properties: DocumentPropertiesData }
  | { readonly status: "failed" };

/** The metadata the Description form edits: once read, or that it is being read or couldn't be. */
export type MetadataState =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly metadata: DocumentMetadataData }
  | { readonly status: "failed" };

/** What the dialog needs to edit the document (O04). */
export interface PropertiesEditing {
  /** The metadata as the edits so far leave it. */
  readonly metadata: MetadataState;
  /** Whether a save rewrites the whole file, so a removed value leaves no earlier version behind. */
  readonly rewritesWhole: boolean;
  /**
   * Apply: `change` as one edit (`replaceMalformed` when the XMP metadata can't be read). Resolves
   * once applied, and the dialog closes; rejects when it couldn't be, and the dialog says so.
   */
  readonly onApply: (change: MetadataChange, options: { readonly replaceMalformed: boolean }) => Promise<void>;
}

/** Props of `PropertiesDialog`. */
export interface PropertiesDialogProps {
  /** Whether the dialog is open. */
  isOpen: boolean;
  /** The open file's name, under the title. */
  fileName: string;
  state: PropertiesState;
  /** The pages' sizes and labels, which the properties don't carry. */
  context: PropertiesContext;
  /** A value or Copy all was pressed: copy `text`, and say so. */
  onCopy: (text: string) => void;
  /** Close, Escape or the scrim. */
  onClose: () => void;
  /** Given when the document can be edited: the Description tab becomes a form. */
  editing?: PropertiesEditing | undefined;
}

function Rows({ section, onCopy }: { section: PropertySection; onCopy: (text: string) => void }): ReactNode {
  return (
    <dl className="paper-properties-list">
      {section.rows.map((row) => (
        <div key={row.label} className="paper-properties-row">
          <dt>{row.label}</dt>
          <dd>
            <span className="paper-properties-value">{row.value}</span>
            <IconButton
              icon="copy"
              size={26}
              label={laterMessage("properties.copy", { label: row.label.toLowerCase() })}
              onPress={() => onCopy(row.value)}
            />
          </dd>
        </div>
      ))}
    </dl>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }): ReactNode {
  return (
    <section className="paper-properties-section" aria-labelledby={id}>
      <h3 id={id} className="paper-properties-heading">
        {title}
      </h3>
      {children}
    </section>
  );
}

function Fonts({ properties }: { properties: DocumentPropertiesData }): ReactNode {
  if (properties.fonts.length === 0)
    return <p className="paper-properties-empty">{laterMessage("properties.fonts.none")}</p>;
  return (
    <table className="paper-properties-fonts">
      <thead>
        <tr>
          <th scope="col">{laterMessage("properties.fonts.name")}</th>
          <th scope="col">{laterMessage("properties.fonts.type")}</th>
          <th scope="col">{laterMessage("properties.fonts.storage")}</th>
        </tr>
      </thead>
      <tbody>
        {properties.fonts.map((font, i) => (
          // Two fonts may share a name (a subset of each weight): the index keeps the rows apart.
          // biome-ignore lint/suspicious/noArrayIndexKey: the list is fixed while the dialog is open.
          <tr key={i}>
            <td>{font.name ?? laterMessage("properties.font.unnamed")}</td>
            <td>{font.kind ?? ""}</td>
            <td>{fontStorage(font)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Body({
  properties,
  context,
  onCopy,
  withDescription = true,
}: {
  properties: DocumentPropertiesData;
  context: PropertiesContext;
  onCopy: (text: string) => void;
  /** Whether to show the Description section: the File tab leaves it to the form. */
  withDescription?: boolean;
}): ReactNode {
  const { description, document, security } = propertySections(properties, context);
  const restricted = properties.encryption?.permissions !== undefined;
  return (
    <div className="paper-properties-body">
      {withDescription && description.rows.length > 0 ? (
        <Section id="paper-properties-description" title={description.title}>
          <Rows section={description} onCopy={onCopy} />
        </Section>
      ) : null}
      <Section id="paper-properties-document" title={document.title}>
        <Rows section={document} onCopy={onCopy} />
      </Section>
      <Section id="paper-properties-fonts" title={laterMessage("properties.section.fonts")}>
        <Fonts properties={properties} />
      </Section>
      <Section id="paper-properties-security" title={security.title}>
        <Rows section={security} onCopy={onCopy} />
        {restricted ? <p className="paper-properties-note">{laterMessage("properties.permissions.note")}</p> : null}
      </Section>
    </div>
  );
}

/** The read-only facts, or that they are being read or couldn't be. */
function Facts({
  state,
  context,
  onCopy,
  withDescription,
}: Pick<PropertiesDialogProps, "state" | "context" | "onCopy"> & { withDescription: boolean }): ReactNode {
  if (state.status === "ready") {
    return <Body properties={state.properties} context={context} onCopy={onCopy} withDescription={withDescription} />;
  }
  return (
    <p className="paper-properties-empty">
      {laterMessage(state.status === "loading" ? "properties.loading" : "properties.failed")}
    </p>
  );
}

type Tab = "description" | "file";

/**
 * The editable dialog's state: the form, with the metadata it started from (new metadata, when
 * the dialog opens again or an undo changed it, starts a new form), what Apply would change,
 * and Apply itself, which closes the dialog once the edit is made.
 */
function useMetadataForm(editing: PropertiesEditing, onClose: () => void) {
  const metadata = editing.metadata.status === "ready" ? editing.metadata.metadata : undefined;
  const [tab, setTab] = useState<Tab>("description");
  const [held, setHeld] = useState<{ from: DocumentMetadataData; form: MetadataForm } | undefined>();
  const [applying, setApplying] = useState(false);
  const [failed, setFailed] = useState(false);
  const form = metadata === undefined ? undefined : held?.from === metadata ? held.form : formFrom(metadata);
  const change = metadata === undefined || form === undefined ? undefined : changeOf(metadata, form);
  const close = () => {
    setHeld(undefined);
    setFailed(false);
    setTab("description");
    onClose();
  };
  const apply = () => {
    if (metadata === undefined || change === undefined) return;
    setApplying(true);
    setFailed(false);
    editing.onApply(change, { replaceMalformed: metadata.packet === "malformed" }).then(
      () => {
        setApplying(false);
        close();
      },
      () => {
        setApplying(false);
        setFailed(true);
      },
    );
  };
  const setForm = (next: MetadataForm) => {
    if (metadata !== undefined) setHeld({ from: metadata, form: next });
  };
  return { metadata, form, change, tab, setTab, applying, failed, close, apply, setForm };
}

/** The editable dialog's footer: Remove metadata… (or Copy all on the File tab), Cancel and Apply. */
function EditFooter({
  start,
  canApply,
  onCancel,
  onApply,
}: {
  start: ReactNode;
  canApply: boolean;
  onCancel: () => void;
  onApply: () => void;
}): ReactNode {
  return (
    <>
      {start === null ? null : <span className="paper-properties-actions-start">{start}</span>}
      <Button onPress={onCancel}>{laterMessage("properties.edit.cancel")}</Button>
      <Button variant="primary" isDisabled={!canApply} onPress={onApply}>
        {laterMessage("properties.edit.apply")}
      </Button>
    </>
  );
}

/** The editable dialog (O04): its form starts from the metadata each time the dialog opens. */
function EditableDialog(props: PropertiesDialogProps & { editing: PropertiesEditing }): ReactNode {
  const { isOpen, fileName, state, context, onCopy, editing } = props;
  const edit = useMetadataForm(editing, props.onClose);
  const { metadata, form } = edit;
  const [removing, setRemoving] = useState(false);
  const removable =
    metadata !== undefined &&
    form !== undefined &&
    EDITABLE_FIELDS.some((f) => hasValue(metadata, f) && !form.removed.includes(f));
  const start =
    edit.tab === "description" ? (
      <Button variant="danger" isDisabled={!removable} onPress={() => setRemoving(true)}>
        {laterMessage("properties.edit.removeMetadata")}
      </Button>
    ) : state.status === "ready" ? (
      <Button variant="ghost" onPress={() => onCopy(propertiesText(state.properties, context))}>
        {laterMessage("properties.copyAll")}
      </Button>
    ) : null;
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) edit.close();
      }}
      size="form"
      title={laterMessage("properties.dialog.title")}
      description={fileName}
      closeLabel={laterMessage("properties.close")}
      actions={
        <EditFooter
          start={start}
          canApply={edit.change !== undefined && !edit.applying}
          onCancel={edit.close}
          onApply={edit.apply}
        />
      }
    >
      <SegmentedTabs
        aria-label={laterMessage("properties.dialog.title")}
        tabs={[
          { id: "description", label: laterMessage("properties.tab.description") },
          { id: "file", label: laterMessage("properties.tab.file") },
        ]}
        selectedKey={edit.tab}
        onSelectionChange={edit.setTab}
      >
        <TabPanel id="description">
          {edit.failed ? <Notice tone="danger" title={laterMessage("properties.edit.failed")} /> : null}
          {metadata !== undefined && form !== undefined ? (
            <PropertiesEditor
              metadata={metadata}
              form={form}
              onForm={edit.setForm}
              removalStays={removalStaysInEarlierVersions(edit.change, editing.rewritesWhole)}
            />
          ) : (
            <p className="paper-properties-empty">
              {laterMessage(editing.metadata.status === "failed" ? "properties.failed" : "properties.loading")}
            </p>
          )}
        </TabPanel>
        <TabPanel id="file">
          <Facts state={state} context={context} onCopy={onCopy} withDescription={false} />
        </TabPanel>
      </SegmentedTabs>
      {metadata !== undefined && form !== undefined ? (
        <RemoveMetadataDialog
          isOpen={removing}
          metadata={metadata}
          form={form}
          onRemove={(fields) => edit.setForm({ ...form, removed: [...form.removed, ...fields] })}
          onClose={() => setRemoving(false)}
        />
      ) : null}
    </Dialog>
  );
}

/** Shows a document's properties; with `editing`, lets the reader change its description. */
export function PropertiesDialog(props: PropertiesDialogProps): ReactNode {
  if (props.editing !== undefined) return <EditableDialog {...props} editing={props.editing} />;
  const { isOpen, fileName, state, context, onCopy, onClose } = props;
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      size="form"
      title={laterMessage("properties.dialog.title")}
      description={fileName}
      closeLabel={laterMessage("properties.close")}
      actions={
        <>
          {state.status === "ready" ? (
            <Button variant="ghost" onPress={() => onCopy(propertiesText(state.properties, context))}>
              {laterMessage("properties.copyAll")}
            </Button>
          ) : null}
          <Button variant="primary" onPress={onClose}>
            {laterMessage("properties.close")}
          </Button>
        </>
      }
    >
      <Facts state={state} context={context} onCopy={onCopy} withDescription />
    </Dialog>
  );
}
