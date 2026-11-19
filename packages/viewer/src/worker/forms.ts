// The open document's form, read by Lamina PDF in the worker (the extract module, as the
// annotations are), for Fill & Sign's form layer: its fields with their widgets and values, and
// the order Tab visits each page's widgets in. Both are kept until an edit, since a value set on
// one field can change any page's widgets (`forgetForm`).

import type { FormFields, TabOrder } from "@laminapdf/sdk";
import { textModel } from "./document.js";

/** The form as last read, until an edit. */
let fields: FormFields | undefined;
/** Each page's tab order as last read, until an edit. */
const orders = new Map<number, TabOrder>();

/** The document's form fields, in the order the form lists them (cached). */
export async function formFields(signal: AbortSignal): Promise<FormFields> {
  if (fields !== undefined) return fields;
  const model = await textModel(signal);
  const read = await model.ensure(() => model.formFields(), { signal });
  fields = read;
  return read;
}

/** The order Tab visits page `index`'s widgets in (cached). */
export async function tabOrder(index: number, signal: AbortSignal): Promise<TabOrder> {
  const cached = orders.get(index);
  if (cached !== undefined) return cached;
  const model = await textModel(signal);
  const read = await model.ensure(() => model.tabOrder(index), { signal });
  orders.set(index, read);
  return read;
}

/** Forgets the form as read: an edit changed it, or the document closed. */
export function forgetForm(): void {
  fields = undefined;
  orders.clear();
}
