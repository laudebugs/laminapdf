// The document properties dialog (R08): everything Lamina PDF's own parser knows about the file,
// in four sections (Description, Document, Fonts, Security), at form width. Each value has a copy
// button named by what it copies, shown on hover or focus; Copy all writes every section as plain
// text. Read-only: nothing here changes the file.

import type { ReactNode } from "react";
import { Button, IconButton } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type DocumentPropertiesData,
  fontStorage,
  type PropertiesContext,
  type PropertySection,
  propertiesText,
  propertySections,
} from "./properties.js";

/** What the dialog shows: the properties once read, or that they are being read or couldn't be. */
export type PropertiesState =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly properties: DocumentPropertiesData }
  | { readonly status: "failed" };

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
              label={message("properties.copy", { label: row.label.toLowerCase() })}
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
    return <p className="paper-properties-empty">{message("properties.fonts.none")}</p>;
  return (
    <table className="paper-properties-fonts">
      <thead>
        <tr>
          <th scope="col">{message("properties.fonts.name")}</th>
          <th scope="col">{message("properties.fonts.type")}</th>
          <th scope="col">{message("properties.fonts.storage")}</th>
        </tr>
      </thead>
      <tbody>
        {properties.fonts.map((font, i) => (
          // Two fonts may share a name (a subset of each weight): the index keeps the rows apart.
          // biome-ignore lint/suspicious/noArrayIndexKey: the list is fixed while the dialog is open.
          <tr key={i}>
            <td>{font.name ?? message("properties.font.unnamed")}</td>
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
}: {
  properties: DocumentPropertiesData;
  context: PropertiesContext;
  onCopy: (text: string) => void;
}): ReactNode {
  const { description, document, security } = propertySections(properties, context);
  const restricted = properties.encryption?.permissions !== undefined;
  return (
    <div className="paper-properties-body">
      {description.rows.length > 0 ? (
        <Section id="paper-properties-description" title={description.title}>
          <Rows section={description} onCopy={onCopy} />
        </Section>
      ) : null}
      <Section id="paper-properties-document" title={document.title}>
        <Rows section={document} onCopy={onCopy} />
      </Section>
      <Section id="paper-properties-fonts" title={message("properties.section.fonts")}>
        <Fonts properties={properties} />
      </Section>
      <Section id="paper-properties-security" title={security.title}>
        <Rows section={security} onCopy={onCopy} />
        {restricted ? <p className="paper-properties-note">{message("properties.permissions.note")}</p> : null}
      </Section>
    </div>
  );
}

/** Shows a document's properties. */
export function PropertiesDialog({
  isOpen,
  fileName,
  state,
  context,
  onCopy,
  onClose,
}: PropertiesDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      size="form"
      title={message("properties.dialog.title")}
      description={fileName}
      closeLabel={message("properties.close")}
      actions={
        <>
          {state.status === "ready" ? (
            <Button variant="ghost" onPress={() => onCopy(propertiesText(state.properties, context))}>
              {message("properties.copyAll")}
            </Button>
          ) : null}
          <Button variant="primary" onPress={onClose}>
            {message("properties.close")}
          </Button>
        </>
      }
    >
      {state.status === "ready" ? <Body properties={state.properties} context={context} onCopy={onCopy} /> : null}
      {state.status === "loading" ? <p className="paper-properties-empty">{message("properties.loading")}</p> : null}
      {state.status === "failed" ? <p className="paper-properties-empty">{message("properties.failed")}</p> : null}
    </Dialog>
  );
}
