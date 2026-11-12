// The XMP tab of the document properties dialog (O04): every property of the XMP metadata in a
// tree of schemas, the selected one's detail with its value and type, Add property… and Remove,
// and the packet as raw XML. The dialog holds the edits; this draws them and reports each one.
// Properties the Description tab edits open there; the PDF/A and PDF/UA identification and the
// extension schemas are kept as they are, since Lamina PDF writes the declarations itself.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useMemo, useState } from "react";
import { Button as AriaButton, type Key, Tree, TreeItem, TreeItemContent } from "react-aria-components";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { Select, type SelectOption, TextArea, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { chooseOnPress, PressedGroup } from "../components/pressed-group.js";
import { ToggleButton } from "../components/toggle.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { pdfaLevel } from "./properties-editor.js";
import type { MetadataField } from "./properties-form.js";
import {
  changeKey,
  isXmlName,
  isXmlPrefix,
  readXmpPacket,
  withChange,
  withoutChange,
  type XmpChange,
  type XmpProperty,
  type XmpSchema,
  type XmpType,
  type XmpValueData,
  xmpValueOf,
} from "./xmp-model.js";

/** The document's XMP metadata as text, as `@laminapdf/sdk`'s `doc.xmpPacket()` gives it. */
export interface XmpPacketData {
  readonly status: "absent" | "parsed" | "malformed";
  readonly xml?: string | undefined;
  /** The PDF/A level it claims, e.g. `"2B"`. */
  readonly pdfa?: string | undefined;
}

/** The tab's edits, and whether they can be applied (the raw XML parses, every value fits its type). */
export interface XmpEdits {
  readonly changes: readonly XmpChange[];
  readonly valid: boolean;
}

/** Props of `XmpEditor`. */
export interface XmpEditorProps {
  readonly packet: XmpPacketData;
  readonly edits: XmpEdits;
  readonly onEdits: (edits: XmpEdits) => void;
  /** A property the Description tab edits was chosen: open that tab. */
  readonly onDescription: (field: MetadataField) => void;
  /** Whether a removal or a replaced packet will leave the old values in the file's earlier versions. */
  readonly removalStays?: boolean;
}

const TYPES: readonly XmpType[] = ["text", "uri", "integer", "real", "boolean", "date", "seq", "bag", "langAlt"];

function typeOptions(): SelectOption<XmpType>[] {
  return TYPES.map((value) => ({ value, label: laterMessage(`properties.xmp.type.${value}`) }));
}

/** A value as the tab shows it: one line, an item per line, or `lang: text` per line. */
function textOf(value: XmpValueData): string {
  switch (value.type) {
    case "langAlt":
      return value.value.map((e) => `${e.lang}: ${e.text}`).join("\n");
    case "seq":
    case "bag":
      return value.value.map((v) => textOf(v)).join("\n");
    case "boolean":
      return value.value ? "True" : "False";
    default:
      return String(value.value);
  }
}

/** The packet's properties with the tab's edits: set values shown, new properties added to their schema. */
function edited(schemas: readonly XmpSchema[], changes: readonly XmpChange[]): XmpSchema[] {
  const out = schemas.map((s) => ({ ...s, properties: [...s.properties] }));
  for (const c of changes) {
    if (c.kind !== "set") continue;
    const shown: XmpProperty = {
      namespace: c.namespace,
      prefix: c.prefix,
      name: c.name,
      text: textOf(c.value),
      type: c.value.type,
      kept: false,
    };
    const schema = out.find((s) => s.namespace === c.namespace);
    if (schema === undefined) {
      out.push({ namespace: c.namespace, prefix: c.prefix, name: undefined, properties: [shown] });
      continue;
    }
    const i = schema.properties.findIndex((p) => p.name === c.name);
    if (i === -1) schema.properties.push(shown);
    else schema.properties[i] = { ...(schema.properties[i] as XmpProperty), text: shown.text, type: shown.type };
  }
  return out;
}

/** A property's value in its tree row: the first line, without a language's `lang: `. */
function preview(p: XmpProperty): string {
  const first = p.text.split("\n")[0] ?? "";
  return p.type === "langAlt" ? first.replace(/^[^:]*:\s?/, "") : first;
}

/** The tree of schemas and properties; a removed property is struck through. */
function PropertyTree({
  schemas,
  removed,
  selected,
  onSelect,
}: {
  schemas: readonly XmpSchema[];
  removed: ReadonlySet<string>;
  selected: string | undefined;
  onSelect: (key: string) => void;
}): ReactNode {
  const [expanded, setExpanded] = useState<Set<Key>>(() => new Set(schemas.map((s) => s.namespace)));
  return (
    <Tree
      aria-label={laterMessage("properties.xmp.tree")}
      className="paper-outline-tree paper-xmp-tree"
      selectionMode="single"
      selectionBehavior="replace"
      disallowEmptySelection
      // Escape closes the dialog, as everywhere else in it, rather than clearing the selection.
      escapeKeyBehavior="none"
      selectedKeys={selected === undefined ? [] : [selected]}
      onSelectionChange={(keys) => {
        const key = keys === "all" ? undefined : [...keys][0];
        if (typeof key === "string" && key.includes(" ")) onSelect(key);
      }}
      expandedKeys={expanded}
      onExpandedChange={setExpanded}
    >
      {schemas.map((schema) => (
        <TreeItem
          key={schema.namespace}
          id={schema.namespace}
          textValue={schema.name ?? schema.prefix}
          className="paper-outline-item"
        >
          <TreeItemContent>
            {({ isExpanded }) => (
              <div className="paper-outline-row" data-level={1}>
                <AriaButton slot="chevron" className="paper-outline-toggle" excludeFromTabOrder>
                  <Icon name={isExpanded ? "chevron-down" : "chevron-right"} />
                </AriaButton>
                <span className="paper-outline-link">
                  <span className="paper-outline-title">{schema.name ?? schema.prefix}</span>
                </span>
              </div>
            )}
          </TreeItemContent>
          {schema.properties.map((p) => {
            const key = changeKey(p);
            return (
              <TreeItem key={key} id={key} textValue={p.name} className="paper-outline-item paper-xmp-item">
                <TreeItemContent>
                  <div className="paper-outline-row" data-level={2}>
                    <span className="paper-outline-link">
                      <span className="paper-outline-title">{removed.has(key) ? <del>{p.name}</del> : p.name}</span>
                      <span className="paper-outline-page paper-xmp-value">{preview(p)}</span>
                    </span>
                  </div>
                </TreeItemContent>
              </TreeItem>
            );
          })}
        </TreeItem>
      ))}
    </Tree>
  );
}

/** The selected property: its schema, and its value and type when the tab may edit it. */
function Detail({
  property,
  removed,
  edits,
  onEdits,
  onDescription,
}: {
  property: XmpProperty;
  removed: boolean;
  edits: XmpEdits;
  onEdits: (edits: XmpEdits) => void;
  onDescription: (field: MetadataField) => void;
}): ReactNode {
  const [draft, setDraft] = useState<{ type: XmpType; text: string } | undefined>();
  const type = draft?.type ?? property.type ?? "text";
  const text = draft?.text ?? property.text;
  const fits = xmpValueOf(type, text) !== undefined;
  const set = (next: { type: XmpType; text: string }) => {
    setDraft(next);
    const value = xmpValueOf(next.type, next.text);
    const { namespace, prefix, name } = property;
    const changes =
      value === undefined ? edits.changes : withChange(edits.changes, { kind: "set", namespace, prefix, name, value });
    onEdits({ changes, valid: value !== undefined });
  };
  const heading = (
    <>
      <h3 className="paper-properties-heading">{`${property.prefix}:${property.name}`}</h3>
      <p className="paper-properties-note">{property.namespace}</p>
    </>
  );
  if (property.field !== undefined) {
    const field = property.field;
    return (
      <div className="paper-xmp-detail">
        {heading}
        <p className="paper-properties-note">{laterMessage("properties.xmp.mapped")}</p>
        <Button onPress={() => onDescription(field)}>{laterMessage("properties.xmp.openDescription")}</Button>
      </div>
    );
  }
  if (property.kept || property.type === undefined) {
    return (
      <div className="paper-xmp-detail">
        {heading}
        <p className="paper-properties-note">
          {laterMessage(property.kept ? "properties.xmp.kept" : "properties.xmp.structure")}
        </p>
      </div>
    );
  }
  const remove = () =>
    onEdits({
      changes: withChange(edits.changes, { kind: "remove", namespace: property.namespace, name: property.name }),
      valid: true,
    });
  const keep = () => onEdits({ changes: withoutChange(edits.changes, changeKey(property)), valid: true });
  const many = type === "seq" || type === "bag" || type === "langAlt";
  const Field = many ? TextArea : TextInput;
  return (
    <div className="paper-xmp-detail">
      {heading}
      {removed ? (
        <div className="paper-properties-removed-row">
          <Badge tone="danger">{laterMessage("properties.edit.willRemove")}</Badge>
          <Button variant="ghost" size={26} onPress={keep}>
            {laterMessage("properties.edit.keep")}
          </Button>
        </div>
      ) : (
        <>
          <Select
            label={laterMessage("properties.xmp.type")}
            options={typeOptions()}
            value={type}
            onChange={(t) => set({ type: t, text })}
          />
          <Field
            label={laterMessage("properties.xmp.value")}
            value={text}
            onChange={(t) => set({ type, text: t })}
            description={laterMessage(`properties.xmp.value.help.${many ? type : "one"}`)}
            {...(fits ? {} : { errorMessage: laterMessage(`properties.xmp.invalid.${type}`) })}
          />
          <Button variant="danger" onPress={remove}>
            {laterMessage("properties.xmp.remove")}
          </Button>
        </>
      )}
    </div>
  );
}

/** Add property…: a namespace, prefix and name, then the value as Detail edits it. */
function AddProperty({ onAdd, onCancel }: { onAdd: (change: XmpChange) => void; onCancel: () => void }): ReactNode {
  const [fields, setFields] = useState({ namespace: "", prefix: "", name: "", type: "text" as XmpType, text: "" });
  const value = xmpValueOf(fields.type, fields.text);
  const namespaceOk = /^[a-z][a-z0-9+.-]*:\S+$/i.test(fields.namespace);
  const ok = namespaceOk && isXmlPrefix(fields.prefix) && isXmlName(fields.name) && value !== undefined;
  const field = (key: "namespace" | "prefix" | "name", label: string, help: string) => (
    <TextInput
      label={label}
      description={help}
      value={fields[key]}
      onChange={(v) => setFields({ ...fields, [key]: v })}
    />
  );
  return (
    <div className="paper-xmp-detail">
      <h3 className="paper-properties-heading">{laterMessage("properties.xmp.add.title")}</h3>
      {field("namespace", laterMessage("properties.xmp.namespace"), laterMessage("properties.xmp.namespace.help"))}
      {field("prefix", laterMessage("properties.xmp.prefix"), laterMessage("properties.xmp.prefix.help"))}
      {field("name", laterMessage("properties.xmp.name"), laterMessage("properties.xmp.name.help"))}
      <Select
        label={laterMessage("properties.xmp.type")}
        options={typeOptions()}
        value={fields.type}
        onChange={(type) => setFields({ ...fields, type })}
      />
      <TextArea
        label={laterMessage("properties.xmp.value")}
        value={fields.text}
        onChange={(text) => setFields({ ...fields, text })}
      />
      <div className="paper-properties-removed-row">
        <Button onPress={onCancel}>{laterMessage("properties.edit.cancel")}</Button>
        <Button
          variant="primary"
          isDisabled={!ok}
          onPress={() => {
            if (value === undefined) return;
            const { namespace, prefix, name } = fields;
            onAdd({ kind: "set", namespace: namespace.trim(), prefix, name, value });
          }}
        >
          {laterMessage("properties.xmp.add.confirm")}
        </Button>
      </div>
    </div>
  );
}

/** The packet as raw XML: edited as text, checked as it changes, replacing the packet when it parses. */
function RawXml({ packet, edits, onEdits }: Pick<XmpEditorProps, "packet" | "edits" | "onEdits">): ReactNode {
  const replaced = edits.changes.find((c) => c.kind === "replace");
  const [text, setText] = useState(replaced?.xml ?? packet.xml ?? "");
  // The XML shown is the packet before the property edits: replacing it would drop them.
  if (edits.changes.some((c) => c.kind !== "replace")) {
    return (
      <>
        <Notice tone="warning" title={laterMessage("properties.xmp.raw.locked")}>
          {laterMessage("properties.xmp.raw.locked.body")}
        </Notice>
        <TextArea label={laterMessage("properties.xmp.raw")} value={text} isReadOnly />
      </>
    );
  }
  const read = readXmpPacket(text);
  const error = read.ok
    ? undefined
    : read.error.line === undefined
      ? read.error.message
      : laterMessage("properties.xmp.raw.error", { line: read.error.line, message: read.error.message });
  return (
    <TextArea
      label={laterMessage("properties.xmp.raw")}
      value={text}
      onChange={(next) => {
        setText(next);
        const ok = readXmpPacket(next).ok;
        const changes = ok && next !== packet.xml ? [{ kind: "replace", xml: next } as const] : [];
        onEdits({ changes, valid: ok });
      }}
      description={laterMessage("properties.xmp.raw.help")}
      {...(error === undefined ? {} : { errorMessage: error })}
    />
  );
}

/** The schemas with the tab's edits, only the properties `filter` matches (by name or value). */
function visible(read: ReturnType<typeof readXmpPacket> | undefined, changes: readonly XmpChange[], filter: string) {
  const all = edited(read?.ok ? read.schemas : [], changes);
  const q = filter.trim().toLowerCase();
  if (q === "") return all;
  return all
    .map((s) => ({
      ...s,
      properties: s.properties.filter((p) => `${p.prefix}:${p.name} ${p.text}`.toLowerCase().includes(q)),
    }))
    .filter((s) => s.properties.length > 0);
}

/** The toolbar: Properties or Raw XML, and on Properties the filter and Add property…. */
function Toolbar({
  view,
  onView,
  filter,
  onFilter,
  canAdd,
  onAdd,
}: {
  view: "properties" | "raw";
  onView: (view: "properties" | "raw") => void;
  filter: string;
  onFilter: (filter: string) => void;
  canAdd: boolean;
  onAdd: () => void;
}): ReactNode {
  return (
    <div className="paper-xmp-toolbar">
      <PressedGroup label={laterMessage("properties.xmp.view")} className="paper-xmp-views">
        <ToggleButton isSelected={view === "properties"} onChange={chooseOnPress("properties", onView)}>
          {laterMessage("properties.xmp.view.properties")}
        </ToggleButton>
        <ToggleButton isSelected={view === "raw"} onChange={chooseOnPress("raw", onView)}>
          {laterMessage("properties.xmp.view.raw")}
        </ToggleButton>
      </PressedGroup>
      {view === "properties" ? (
        <>
          <TextInput label={laterMessage("properties.xmp.filter")} value={filter} onChange={onFilter} />
          <Button isDisabled={!canAdd} onPress={onAdd}>
            {laterMessage("properties.xmp.add")}
          </Button>
        </>
      ) : null}
    </div>
  );
}

/** The tree beside the selected property's detail, or Add property…. */
function Panes({
  schemas,
  empty,
  edits,
  onEdits,
  onDescription,
  adding,
  onAdding,
}: Pick<XmpEditorProps, "edits" | "onEdits" | "onDescription"> & {
  schemas: readonly XmpSchema[];
  /** The file has no XMP metadata and nothing was added. */
  empty: boolean;
  adding: boolean;
  onAdding: (adding: boolean) => void;
}): ReactNode {
  const [selected, setSelected] = useState<string | undefined>();
  const removed = new Set(edits.changes.flatMap((c) => (c.kind === "remove" ? [changeKey(c)] : [])));
  const property = schemas.flatMap((s) => s.properties).find((p) => changeKey(p) === selected);
  const detail = adding ? (
    <AddProperty
      onCancel={() => onAdding(false)}
      onAdd={(change) => {
        onEdits({ changes: withChange(edits.changes, change), valid: true });
        onAdding(false);
        if (change.kind === "set") setSelected(changeKey(change));
      }}
    />
  ) : property === undefined ? (
    <p className="paper-properties-empty">{laterMessage("properties.xmp.choose")}</p>
  ) : (
    <Detail
      key={selected}
      property={property}
      removed={removed.has(changeKey(property))}
      edits={edits}
      onEdits={onEdits}
      onDescription={onDescription}
    />
  );
  return (
    <div className="paper-xmp-panes">
      {empty ? (
        <p className="paper-properties-empty">{laterMessage("properties.xmp.none")}</p>
      ) : (
        <PropertyTree schemas={schemas} removed={removed} selected={selected} onSelect={setSelected} />
      )}
      {detail}
    </div>
  );
}

/** The XMP tab. */
export function XmpEditor({ packet, edits, onEdits, onDescription, removalStays = false }: XmpEditorProps): ReactNode {
  const [view, setView] = useState<"properties" | "raw">(() =>
    edits.changes.some((c) => c.kind === "replace") || packet.status === "malformed" ? "raw" : "properties",
  );
  const [filter, setFilter] = useState("");
  const [adding, setAdding] = useState(false);
  const read = useMemo(() => (packet.xml === undefined ? undefined : readXmpPacket(packet.xml)), [packet.xml]);
  const replacing = edits.changes.some((c) => c.kind === "replace");
  const schemas = useMemo(() => visible(read, edits.changes, filter), [read, edits.changes, filter]);
  const count = edits.changes.filter((c) => c.kind !== "replace").length;
  return (
    <div className="paper-xmp">
      <Toolbar
        view={view}
        onView={setView}
        filter={filter}
        onFilter={setFilter}
        canAdd={!replacing}
        onAdd={() => setAdding(true)}
      />
      {removalStays ? (
        <Notice tone="warning" title={laterMessage("properties.edit.notice.earlier")}>
          {laterMessage("properties.edit.notice.earlier.body")}
        </Notice>
      ) : null}
      {packet.pdfa === undefined ? null : (
        <Notice title={laterMessage("properties.xmp.pdfa", { level: pdfaLevel(packet.pdfa) })}>
          {laterMessage("properties.xmp.pdfa.body")}
        </Notice>
      )}
      {view === "raw" ? (
        <RawXml packet={packet} edits={edits} onEdits={onEdits} />
      ) : replacing ? (
        <Notice tone="warning" title={laterMessage("properties.xmp.replacing")}>
          {laterMessage("properties.xmp.replacing.body")}
        </Notice>
      ) : (
        <Panes
          schemas={schemas}
          empty={packet.status === "absent" && count === 0}
          edits={edits}
          onEdits={onEdits}
          onDescription={onDescription}
          adding={adding}
          onAdding={setAdding}
        />
      )}
      {count === 0 ? null : (
        <p className="paper-properties-note" aria-live="polite">
          {laterCountMessage(count, { one: "properties.xmp.pending.one", many: "properties.xmp.pending.many" })}
        </p>
      )}
    </div>
  );
}
