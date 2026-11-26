// Form data in the worker (Paper P03): the open document's answers as a JSON, XFDF or FDF file,
// and a form data file read into the `setFieldValue` ops it would make. Import is a transaction of
// ordinary ops, never an op of its own (E8 plan §10, question 4), so it records, undoes and replays
// like every other edit. The library reads the file on a scratch copy of the document as it is
// now; the answers that differ become the ops, and the copy is let go.

import type { FormDataFormat, FormDataReport, FormField, Op } from "@laminapdf/sdk";
import { Document } from "@laminapdf/sdk";
import type { FormImportPlan } from "../protocol.js";
import { current, extractModule, textModel } from "./document.js";
import { serial } from "./edit.js";

/** Document `id`'s answers as a file in `format`, as its edits leave them; the PDF doesn't change. */
export function exportFormData(
  format: FormDataFormat,
  file: string | undefined,
  id: number,
  signal: AbortSignal,
): Promise<Uint8Array> {
  return serial(current(id), async () => {
    const model = await textModel(signal);
    return model.ensure(() => model.exportFormData(format, file), { signal });
  });
}

/** A field's value as the plan compares it. */
function same(a: unknown, b: unknown): boolean {
  return JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
}

/** The `setFieldValue` ops that take `before`'s fields to `after`'s values. */
function valueOps(before: readonly FormField[], after: readonly FormField[]): Op[] {
  const was = new Map(before.map((field) => [field.id, field]));
  const ops: Op[] = [];
  for (const field of after) {
    if (same(was.get(field.id)?.value, field.value)) continue;
    const value = field.value ?? { type: "clear" };
    ops.push({ op: "setFieldValue", args: { field: field.id, value } } as Op);
  }
  return ops;
}

/**
 * What importing `bytes` (JSON, XFDF or FDF) into document `id` would do: the ops that set the
 * answers it changes, in one transaction, and the report of the names it didn't find, the fields
 * a signature locks and the values the fields refused. Throws the library's error for a file it
 * can't read (`interchange-malformed`, `interchange-unknown-format`).
 */
export function planFormImport(bytes: Uint8Array, id: number, signal: AbortSignal): Promise<FormImportPlan> {
  return serial(current(id), async () => {
    const model = await textModel(signal);
    const { bytes: saved } = await model.ensure(() => model.saveCopy({ mode: "auto", invalidateSignatures: true }), {
      signal,
    });
    const before = await model.ensure(() => model.formFields(), { signal });
    await extractModule();
    const scratch = await Document.open(saved);
    try {
      const report: FormDataReport = scratch.importFormData(bytes);
      const after = scratch.formFields();
      return {
        ops: valueOps(before.fields, after.fields),
        unknown: report.unknown,
        locked: report.locked,
        refused: report.refused,
        xfaNotUpdated: report.xfaNotUpdated,
      };
    } finally {
      scratch.free();
    }
  });
}
