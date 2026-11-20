// Flatten a filled invoice form for sending: every answer becomes part of its page, so the copy
// shows the same everywhere and the answers can no longer be changed.
// The fields go from the form; signature fields, and fields a signature locks, stay. Saving
// as a full rewrite leaves no earlier revision that still holds the fields.
import { Document, type FormDataJson } from "@laminapdf/sdk";

/**
 * The form filled from `answers`, then every field drawn into its page, as a printer would show
 * it. Needs `init({ extract: true })`.
 */
export function flattenForm(form: Uint8Array, answers: FormDataJson): Uint8Array<ArrayBuffer> {
  using doc = Document.open(form);
  doc.fillForm(answers);
  const edited = doc.flattenForm(undefined, "print");
  // A push button or radio button with no appearance of its own stays.
  for (const warning of edited.warnings ?? []) {
    console.warn(warning.code);
  }
  // A full rewrite keeps only the flattened pages, not the revisions with the fields.
  return doc.save({ mode: "fullRewrite" });
}
