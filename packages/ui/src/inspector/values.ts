// What the object inspector (O15) shows, as data: PDF values and objects as `@laminapdf/sdk`
// gives them (the same shapes, so its records pass straight in), the tree's rows built from them,
// and the words for a value's type and an object's kind. No React here.

import { laterCountMessage, laterMessage } from "../later-messages.js";

/** A PDF value by its `type` (ISO 32000-2 §7.3), as `@laminapdf/sdk`'s `CosValue`. */
export type InspectValue =
  | { readonly type: "null" }
  | { readonly type: "boolean"; readonly value: boolean }
  | { readonly type: "integer"; readonly value: number }
  | { readonly type: "real"; readonly value: number }
  | { readonly type: "string"; readonly text: string; readonly writtenAsHex: boolean }
  | { readonly type: "name"; readonly value: string }
  | { readonly type: "array"; readonly items: readonly InspectValue[] }
  | { readonly type: "dictionary"; readonly entries: readonly (readonly [string, InspectValue])[] }
  | { readonly type: "reference"; readonly num: number; readonly generation: number };

/** One object of the document, as `@laminapdf/sdk`'s `ObjectEntry`. */
export interface InspectObjectEntry {
  readonly num: number;
  readonly generation: number;
  /** `dictionary`, `stream`, `array`… */
  readonly kind: string;
  /** Its `/Type`. */
  readonly typeName?: string | undefined;
  /** Its `/Subtype`. */
  readonly subtype?: string | undefined;
  /** Where it is stored. */
  readonly location:
    | { readonly kind: "offset"; readonly offset: number }
    | { readonly kind: "objectStream"; readonly stream: number; readonly index: number }
    | { readonly kind: "created" };
}

/** One object with its value, what refers to it and its PDF syntax. */
export interface InspectObjectData {
  readonly entry: InspectObjectEntry;
  /** Its value (a stream's dictionary for a stream). */
  readonly value: InspectValue;
  /** A stream's filters and stored length. */
  readonly stream?: { readonly filters: readonly string[]; readonly length: number } | undefined;
  /** The 0-based revision that last wrote it. */
  readonly revision?: number | undefined;
  /** The places that refer to it: the object (absent for the trailer) and the path in it. */
  readonly referrers: readonly { readonly from?: number | undefined; readonly path: string }[];
}

/** One row of the object tree. */
export interface TreeRow {
  /** The path from the trailer, unique in the tree ("trailer/Root/Pages/Kids[2]"). */
  readonly id: string;
  /** The key or index ("/Root", "[2]"), or "Trailer". */
  readonly label: string;
  /** What it is, in a few words ("1 0 R · Catalog", "4 keys"). */
  readonly description: string;
  /** The object the row stands for or sits in. */
  readonly object: number | undefined;
  /** The value, when it is one that has children (a reference, an array, a dictionary). */
  readonly value: InspectValue | undefined;
  /** Its children: given only once the row is expanded; `undefined` for a leaf. */
  readonly children: readonly TreeRow[] | undefined;
  /** A row that stands for children still being read. */
  readonly loading?: boolean;
}

/** `ref` in PDF syntax: "12 0 R". */
export function refText(num: number, generation = 0): string {
  return `${num} ${generation} R`;
}

/** A plain description of object `entry`: "Page dictionary", "Font", "Content stream". */
export function objectKind(entry: InspectObjectEntry | undefined): string {
  if (entry === undefined) return laterMessage("inspect.kind.missing");
  const type = entry.typeName;
  if (type === "Page") return laterMessage("inspect.kind.page");
  if (type === "Pages") return laterMessage("inspect.kind.pages");
  if (type === "XObject" && entry.subtype === "Image") return laterMessage("inspect.kind.image");
  if (type !== undefined) return type;
  if (entry.kind === "stream") return laterMessage("inspect.kind.stream");
  return laterMessage(`inspect.type.${entry.kind}` as Parameters<typeof laterMessage>[0]);
}

/** The words for `value`'s type, with its size for arrays and dictionaries: "Array · 4". */
export function typeLabel(value: InspectValue): string {
  const name = laterMessage(`inspect.type.${value.type}`);
  if (value.type === "array") return `${name} · ${value.items.length}`;
  if (value.type === "dictionary") return `${name} · ${value.entries.length}`;
  return name;
}

/** `value` on one line in PDF syntax, cut at `max` characters: `[0 0 612 792]`, `<< /Font /XObject >>`. */
function inlineValue(value: InspectValue, max = 120): string {
  const text = inline(value);
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}

function inline(value: InspectValue): string {
  switch (value.type) {
    case "null":
      return "null";
    case "boolean":
    case "integer":
    case "real":
      return String(value.value);
    case "string":
      return `(${value.text})`;
    case "name":
      return `/${value.value}`;
    case "reference":
      return refText(value.num, value.generation);
    case "array":
      return `[${value.items.map(inline).join(" ")}]`;
    case "dictionary":
      return `<< ${value.entries.map(([key]) => `/${key}`).join(" ")} >>`;
  }
}

/** What a row's value is, in a few words, given what the document's objects are. */
function describe(value: InspectValue, entries: ReadonlyMap<number, InspectObjectEntry>, page?: number): string {
  if (value.type === "reference") {
    const entry = entries.get(value.num);
    const ref = refText(value.num, value.generation);
    if (entry === undefined) return `${ref} · ${laterMessage("inspect.kind.missing")}`;
    if (page !== undefined) return `${ref} · ${laterMessage("inspect.page", { page: page + 1 })}`;
    return entry.typeName === undefined && entry.kind !== "stream" ? ref : `${ref} · ${objectKind(entry)}`;
  }
  if (value.type === "array") {
    return laterCountMessage(value.items.length, { one: "inspect.item", many: "inspect.items" });
  }
  if (value.type === "dictionary") {
    return laterCountMessage(value.entries.length, { one: "inspect.key.one", many: "inspect.keys" });
  }
  return inlineValue(value, 40);
}

/** Whether `value` has children in the tree. */
function hasChildren(value: InspectValue): boolean {
  return (
    value.type === "reference" ||
    (value.type === "array" && value.items.length > 0) ||
    (value.type === "dictionary" && value.entries.length > 0)
  );
}

/** What the tree builder needs to know about the document. */
export interface TreeSource {
  /** Every object, by number. */
  readonly entries: ReadonlyMap<number, InspectObjectEntry>;
  /** The objects read so far, by number. */
  readonly values: ReadonlyMap<number, InspectValue>;
  /** The 0-based page an object number is, if it is one. */
  readonly pageOf: (num: number) => number | undefined;
  /** The rows that are expanded, by id. */
  readonly expanded: ReadonlySet<string>;
}

/** The children of `value` (its entries, its items, or what a reference points to). */
function childValues(
  value: InspectValue,
  source: TreeSource,
): readonly (readonly [string, InspectValue])[] | "loading" {
  if (value.type === "reference") {
    const target = source.values.get(value.num);
    return target === undefined ? "loading" : childValues(target, source);
  }
  if (value.type === "array") return value.items.map((item, i) => [`[${i}]`, item] as const);
  if (value.type === "dictionary") return value.entries.map(([key, item]) => [`/${key}`, item] as const);
  return [];
}

/** The row for `value` at `id`, its children built only when it is expanded. */
function treeRow(
  id: string,
  label: string,
  value: InspectValue,
  container: number | undefined,
  source: TreeSource,
): TreeRow {
  const object = value.type === "reference" ? value.num : container;
  const page = value.type === "reference" ? source.pageOf(value.num) : undefined;
  const description = describe(value, source.entries, page);
  if (!hasChildren(value)) return { id, label, description, object, value: undefined, children: undefined };
  if (!source.expanded.has(id)) {
    // A collapsed row with children: one placeholder, so the tree draws its chevron.
    const placeholder = { id: `${id}/…`, label: "…", description: "", object, value: undefined, children: undefined };
    return { id, label, description, object, value, children: [placeholder] };
  }
  const children = childValues(value, source);
  if (children === "loading") {
    const loading = {
      id: `${id}/…`,
      label: laterMessage("inspect.loading"),
      description: "",
      object,
      value: undefined,
    };
    return { id, label, description, object, value, children: [{ ...loading, children: undefined, loading: true }] };
  }
  return {
    id,
    label,
    description,
    object,
    value,
    children: children.map(([key, child]) => treeRow(`${id}/${key}`, key, child, object, source)),
  };
}

/** The tree's roots: the trailer, then each object gone to by number. */
export function treeRows(
  trailer: InspectValue | undefined,
  goneTo: readonly number[],
  source: TreeSource,
): readonly TreeRow[] {
  const roots: TreeRow[] = [];
  if (trailer !== undefined) {
    const row = treeRow("trailer", laterMessage("inspect.trailer"), trailer, undefined, source);
    roots.push({ ...row, description: "" });
  }
  for (const num of goneTo) {
    const ref: InspectValue = { type: "reference", num, generation: source.entries.get(num)?.generation ?? 0 };
    const row = treeRow(`object-${num}`, laterMessage("inspect.object", { num }), ref, num, source);
    roots.push({ ...row, description: objectKind(source.entries.get(num)) });
  }
  return roots;
}

/** The path of row `id` as breadcrumb steps: each step's row id and label. */
export function pathOf(id: string, rows: readonly TreeRow[]): readonly TreeRow[] {
  for (const row of rows) {
    if (row.id === id) return [row];
    if (row.children !== undefined && id.startsWith(`${row.id}/`)) {
      const below = pathOf(id, row.children);
      if (below.length > 0) return [row, ...below];
    }
  }
  return [];
}
