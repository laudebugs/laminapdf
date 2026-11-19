// Fill an invoice form from JSON: the answers come from an order system, go into the form as one
// change, and the form's own calculations work out each line's amount and the total.
//
// The answers are `{ fields: { "<field name>": value } }`. Fields are matched by their full names
// (`customer.name` is `name` inside `customer`). A check box takes `true` or `false`, a list an
// array of its options. Names the form doesn't have, fields a signature locks and values a field
// refuses are listed; every other answer is filled.
import { Document, type FormDataJson } from "@laminapdf/sdk";

/** The invoice form filled from `answers`, saved after the original bytes. Needs `init({ extract: true })`. */
export function fillInvoice(form: Uint8Array, answers: FormDataJson): Uint8Array<ArrayBuffer> {
  using doc = Document.open(form);
  const { unknown, refused } = doc.fillForm(answers);
  for (const name of unknown) console.warn(`no field named ${name}`);
  for (const field of refused) console.warn(`${field.name} refused its value (${field.code})`);
  const total = doc.formFields().fields.find((f) => f.name === "total")?.value;
  console.log("total:", total);
  return doc.save({ mode: "incremental" });
}
