// The reader's words, kept in one catalogue so that components and the platform never hold text
// of their own: a translation is a second catalogue of the same shape. English only in v0.1.

const EN = {
  "platform.enable.savePicker": "Use a browser that can save to your files, such as Chrome or Edge.",
  "platform.enable.reopen": "Use a browser that can keep access to files, such as Chrome or Edge.",
  "platform.enable.install": "Install Lamina PDF to open files from your computer with it.",
  "platform.picker.pdf": "PDF documents",
  "platform.error.notWritable": "This file can't be saved in place.",
  "platform.error.externalScheme": "Only web and email links open outside Lamina PDF.",
  "platform.error.print": "Printing isn't available yet.",
} as const;

/** A message's key in the catalogue. */
export type MessageId = keyof typeof EN;

/** A catalogue: every message, in one language. */
export type Catalogue = { readonly [Id in MessageId]: string };

/** The English catalogue. */
export const ENGLISH: Catalogue = EN;

/**
 * The message `id` from `catalogue`, with each `{name}` replaced by `values[name]`. A placeholder
 * without a value stays as written, so a missing value shows up rather than vanishing.
 */
export function message(
  id: MessageId,
  values: Readonly<Record<string, string | number>> = {},
  catalogue = ENGLISH,
): string {
  return catalogue[id].replace(/\{(\w+)\}/g, (whole, name: string) => (name in values ? String(values[name]) : whole));
}
