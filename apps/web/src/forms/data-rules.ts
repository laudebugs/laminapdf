// Form data's rules, with nothing to draw: which fields a reset or a flatten reaches,
// the ops each makes, which answers an export lists, and what an import's report says. The controller
// applies them; they run in Node with the app's tests.

import type { FieldValue, FormField, Op } from "@laminapdf/sdk";
import { type FieldsNote, laterCountMessage, laterMessage } from "@laminapdf/ui/editing";
import type { FormImportPlan } from "@laminapdf/viewer";
import { isFilled } from "./rules.ts";

/** The kinds of field a form's reset gives an answer back to: not push buttons or signatures. */
function resettable(field: FormField): boolean {
  return field.kind !== "pushButton" && field.kind !== "signature" && !field.readOnly;
}

/** Whether two values are the same answer. */
function sameValue(a: FieldValue | undefined, b: FieldValue | undefined): boolean {
  return JSON.stringify(a ?? { type: "clear" }) === JSON.stringify(b ?? { type: "clear" });
}

/** What a reset reaches: how many fields it names, how many of those a signature locks, and the ops. */
export interface ResetPlan {
  /** The fields a reset covers, the locked ones too: "Reset all 27 fields?". */
  readonly fields: number;
  /** The covered fields a signature locks, which stay as they are. */
  readonly locked: number;
  /** One `setFieldValue` per field whose answer isn't its default already. */
  readonly ops: Op[];
}

/** The reset of `fields`: each takes its default value (`/DV`), or none (§12.7.6.3); `locks` are left as they are. */
export function planReset(fields: readonly FormField[], locks: ReadonlyMap<string, string>): ResetPlan {
  const covered = fields.filter(resettable);
  const locked = covered.filter((f) => locks.has(f.id));
  const ops = covered
    .filter((f) => !locks.has(f.id))
    .filter((f) => !sameValue(f.value, f.defaultValue))
    .map((f) => ({ op: "setFieldValue", args: { field: f.id, value: f.defaultValue ?? { type: "clear" } } }) as Op);
  return { fields: covered.length, locked: locked.length, ops };
}

/** What a flatten reaches. */
export interface FlattenPlan {
  /** The IDs of the fields drawn into their pages: every field but signature fields and the locked ones. */
  readonly ids: readonly string[];
  /** The fields a signature locks, which are left as they are. */
  readonly locked: number;
  /** Whether a signature field stays a field. */
  readonly hasSignatureField: boolean;
  /** The op, or undefined when no field is left to flatten. */
  readonly op: Op | undefined;
}

/** The flatten of the form: every field but signature fields and those a signature locks. */
export function planFlatten(fields: readonly FormField[], locks: ReadonlyMap<string, string>): FlattenPlan {
  const flat = fields.filter((f) => f.kind !== "signature");
  const ids = flat.filter((f) => !locks.has(f.id)).map((f) => f.id);
  return {
    ids,
    locked: flat.length - ids.length,
    hasSignatureField: fields.some((f) => f.kind === "signature"),
    op: ids.length === 0 ? undefined : ({ op: "flattenForm", args: { fields: ids, intent: "view" } } as Op),
  };
}

/** How many fields have an answer, and how many there are, as the export dialog counts them. */
export function exportCounts(fields: readonly FormField[]): { filled: number; total: number } {
  const listed = fields.filter((f) => f.kind !== "signature" && f.kind !== "pushButton");
  return { filled: listed.filter(isFilled).length, total: listed.length };
}

/** Whether a JSON export's value is an answer: not empty, not an unticked box. */
function answered(value: unknown): boolean {
  if (value === null || value === undefined || value === false || value === "") return false;
  return !(Array.isArray(value) && value.length === 0);
}

/** An export in JSON with only the fields that have an answer. */
export function keepFilledJson(bytes: Uint8Array): Uint8Array {
  const data = JSON.parse(new TextDecoder().decode(bytes)) as { fields?: Record<string, unknown> };
  const kept = Object.entries(data.fields ?? {}).filter(([, value]) => answered(value));
  return new TextEncoder().encode(`${JSON.stringify({ ...data, fields: Object.fromEntries(kept) }, null, 2)}\n`);
}

/** An XFDF export with only the fields that have an answer (a field's value is its `value` element's text). */
export function keepFilledXfdf(bytes: Uint8Array, parse: (xml: string) => Document): Uint8Array {
  const xml = parse(new TextDecoder().decode(bytes));
  // A leaf is a field with no field inside: it is kept when it has a value; parents stay while a child does.
  const walk = (element: Element): boolean => {
    const children = [...element.children].filter((c) => c.localName === "field");
    if (children.length === 0) {
      return [...element.children].some(
        (c) => c.localName === "value" && (c.textContent ?? "") !== "" && c.textContent !== "Off",
      );
    }
    let kept = false;
    for (const child of children) {
      if (walk(child)) kept = true;
      else child.remove();
    }
    return kept;
  };
  for (const fields of [...xml.getElementsByTagName("fields")]) {
    for (const field of [...fields.children].filter((c) => c.localName === "field")) {
      if (!walk(field)) field.remove();
    }
  }
  return new TextEncoder().encode(new XMLSerializer().serializeToString(xml));
}

/** The toast after an import: "19 fields filled · 2 names not in this form". */
export function importMessage(plan: FormImportPlan): string {
  const count = (list: readonly unknown[], one: LaterCount, many: LaterCount) =>
    list.length === 0 ? [] : [laterCountMessage(list.length, { one, many })];
  return [
    ...count(plan.ops, "form.import.filled.one", "form.import.filled.many"),
    ...count(plan.unknown, "form.import.unknown.one", "form.import.unknown.many"),
    ...count(plan.locked, "form.import.locked.one", "form.import.locked.many"),
    ...count(plan.refused, "form.import.refused.one", "form.import.refused.many"),
  ].join(" · ");
}

type LaterCount = Parameters<typeof laterCountMessage>[1]["one"];

/** The names in `list`, the first five. */
function named(list: readonly string[]): string {
  return list.slice(0, 5).join(", ") + (list.length > 5 ? "…" : "");
}

/** The Fields tab's note after an import that changed something: the names it left out; none when it left none. */
export function importNote(file: string, plan: FormImportPlan): FieldsNote | undefined {
  const lines = [
    ...(plan.unknown.length > 0 ? [laterMessage("form.import.unknownList", { names: named(plan.unknown) })] : []),
    ...(plan.locked.length > 0 ? [laterMessage("form.import.lockedList", { names: named(plan.locked) })] : []),
    ...(plan.refused.length > 0
      ? [laterMessage("form.import.refusedList", { names: named(plan.refused.map((r) => r.name)) })]
      : []),
  ];
  if (lines.length === 0) return undefined;
  return { tone: "neutral", title: laterMessage("form.import.noteTitle", { file }), body: lines.join(" ") };
}

/** The Fields tab's note after an import that changed nothing. */
export function importNothing(file: string, plan: FormImportPlan): FieldsNote {
  const unknown = plan.unknown.length;
  const left = importNote(file, plan)?.body;
  const body =
    unknown > 0
      ? laterMessage("form.import.noMatch", { count: unknown, file })
      : laterMessage("form.import.nothing", { file });
  return {
    tone: "warning",
    title: laterMessage(unknown > 0 ? "form.import.noMatchTitle" : "form.import.nothingTitle"),
    body: left === undefined ? body : `${body} ${left}`,
  };
}
