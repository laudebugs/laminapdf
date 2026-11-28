// Fill & Sign's rules, with nothing to draw: the form's tab order, what a probe of the signatures
// allows each field, the required count. The controller applies them; they import no package
// value, so they run in Node with the app's tests. The form layer (@laminapdf/viewer) decides the
// same two things for itself, which field kinds a person fills and which widgets a page shows;
// keep them alike.

import type { FormField, FormWidget, Op, Violation } from "@laminapdf/sdk";
import { laterMessage, type XfaNoticeKind } from "@laminapdf/ui/editing";
import type { FieldAccess } from "@laminapdf/viewer";

/** The kinds of field a person fills: not push buttons or signatures. */
const FILLABLE: ReadonlySet<FormField["kind"]> = new Set(["text", "checkBox", "radioGroup", "comboBox", "listBox"]);
/** Annotation `/F` bits that keep a widget off the page (ISO 32000-2 §12.5.3, Table 167): Hidden, NoView. */
const OFF_PAGE = (1 << 1) | (1 << 5);

/** Whether a field of `kind` is one a person fills. */
function isFillable(kind: FormField["kind"]): boolean {
  return FILLABLE.has(kind);
}

/** Whether `widget` shows on its page. */
function isShown(widget: FormWidget): boolean {
  return widget.page !== undefined && widget.rect !== undefined && (widget.flags & OFF_PAGE) === 0;
}

/** A field's name as a person reads it: its tooltip (`/TU`), else its name. */
export function fieldLabel(field: FormField): string {
  return field.tooltip ?? field.name;
}

/** One stop of the form's tab order: a widget, its field and its page. */
export interface TabStop {
  readonly widget: string;
  readonly field: string;
  readonly page: number;
}

/** Why the form asks before changes: a banner while filling. */
export interface FormBanner {
  readonly kind: "certified" | "usage-rights";
  readonly violations: readonly Violation[];
}

/** Whether `field` holds a value. */
export function isFilled(field: FormField): boolean {
  const value = field.value;
  if (value === undefined) return false;
  if (value.type === "text") return value.value !== "";
  if (value.type === "button") return value.value !== null;
  if (value.type === "choice") return value.value.length > 0;
  return false;
}

/** Required fields filled and in all. */
export function requiredCount(fields: readonly FormField[]): { filled: number; total: number } {
  const required = fields.filter((f) => f.required && isFillable(f.kind));
  return { filled: required.filter(isFilled).length, total: required.length };
}

/**
 * The form's tab order across pages: each page's widgets in the order the page asks for, a radio
 * group once (arrow keys move within it), only the kinds a person fills and only widgets the page
 * shows.
 */
export function tabStops(
  fields: readonly FormField[],
  pages: ReadonlyMap<number, readonly { readonly widget: string; readonly field: string }[]>,
): TabStop[] {
  const byId = new Map(fields.map((f) => [f.id, f]));
  const stops: TabStop[] = [];
  const seen = new Set<string>();
  for (const page of [...pages.keys()].sort((a, b) => a - b)) {
    for (const { widget, field } of pages.get(page) ?? []) {
      const f = byId.get(field);
      if (f === undefined || !stopsAt(f, widget)) continue;
      if (f.kind === "radioGroup" && seen.has(f.id)) continue;
      seen.add(f.id);
      stops.push({ widget, field, page });
    }
  }
  return stops;
}

/** Whether Tab stops at widget `widget` of field `field`: one a person fills, that the page shows. */
function stopsAt(field: FormField, widget: string): boolean {
  const shown = field.widgets.find((w) => w.id === widget);
  return isFillable(field.kind) && shown !== undefined && isShown(shown);
}

/** The banner a probe's `violations` call for: a certification that permits no change, or usage rights that don't cover filling. */
function bannerFrom(violations: readonly Violation[]): FormBanner | undefined {
  if (violations.some((v) => v.restriction === "docMdp" && v.permission === 1)) {
    return { kind: "certified", violations };
  }
  if (violations.some((v) => v.restriction === "usageRights")) return { kind: "usage-rights", violations };
  return undefined;
}

/** The fields a signature locks, by field ID, each with the words that name the signature. */
function locksFrom(fields: readonly FormField[], violations: readonly Violation[]): Map<string, string> {
  const names = new Map(fields.map((f) => [f.id, f.name]));
  const locks = new Map<string, string>();
  for (const v of violations) {
    const locking = v.restriction === "fieldMdp" || v.restriction === "signedWidget";
    if (!locking || v.field === undefined) continue;
    const signature = v.signature.field === undefined ? undefined : names.get(v.signature.field);
    locks.set(
      v.field,
      signature === undefined ? laterMessage("form.locked.unnamed") : laterMessage("form.locked", { signature }),
    );
  }
  return locks;
}

/** Each field's access from a probe's `violations`, and the banner they call for. */
export function accessFrom(
  fields: readonly FormField[],
  violations: readonly Violation[],
): { access: Map<string, FieldAccess>; locks: Map<string, string>; banner: FormBanner | undefined } {
  const banner = bannerFrom(violations);
  const locks = locksFrom(fields, violations);
  const access = new Map<string, FieldAccess>();
  for (const [field, reason] of locks) access.set(field, { kind: "locked", reason });
  if (banner !== undefined) {
    for (const f of fields) if (!access.has(f.id)) access.set(f.id, { kind: "guarded" });
  }
  return { access, locks, banner };
}

/**
 * The ops that ask what the signatures allow: each fillable field given the value it has. A file
 * with none still asks, with a metadata change, so a certification that permits no change shows
 * its banner on entering Fill & Sign (P01).
 */
export function probeOps(fields: readonly FormField[]): Op[] {
  const fillable = fields.filter((f) => isFillable(f.kind));
  if (fillable.length === 0) return [{ op: "setInfo", args: { patch: { title: "" } } } as Op];
  return fillable.map(
    (f) => ({ op: "setFieldValue", args: { field: f.id, value: f.value ?? { type: "clear" } } }) as Op,
  );
}

/** Whether a signature field of `fields` holds a signature. */
export function hasSignedSignature(fields: readonly FormField[]): boolean {
  return fields.some((f) => f.kind === "signature" && f.signed);
}

/**
 * The notice a form's XFA part calls for, from its `kind` (undefined: none): dynamic forms need
 * Adobe's apps; a static one that is `held` (a signed signature field, or the save planner says
 * removing the XFA part would break a signature) keeps its XFA part; once the reader took the part
 * out, `removed` says so.
 */
export function xfaNotice(
  kind: "static" | "dynamic" | undefined,
  held: boolean,
  removed: boolean,
): XfaNoticeKind | undefined {
  if (kind === undefined) return removed ? "removed" : undefined;
  if (kind === "dynamic") return "dynamic";
  return held ? "signed" : "static";
}
