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
  "reader.sidebar": "Sidebar",
  "reader.noDocument": "No document open",
  "reader.loadingPages": "Loading pages…",
  "reader.locked": "Locked",
  "reader.page": "Page",
  "reader.page.total": "/ {total}",
  "reader.page.position": "({page} / {total})",
  "reader.page.invalid": "There is no page {value}. Enter a page from 1 to {total}.",
  "reader.zoom": "Zoom",
  "reader.zoom.current": "Zoom, {percent}%",
  "reader.zoom.percent": "{percent}%",
  "reader.zoom.out": "Zoom out",
  "reader.zoom.in": "Zoom in",
  "reader.zoom.fitWidth": "Fit width",
  "reader.zoom.fitPage": "Fit page",
  "reader.zoom.actualSize": "Actual size",
  "reader.commands": "Commands",
  "reader.more": "More",
  "reader.password.title": "Enter the password",
  "reader.password.body": "{name} is encrypted. Enter its password to show the pages.",
  "reader.password.field": "Password",
  "reader.password.show": "Show",
  "reader.password.showName": "Show password",
  "reader.password.help": "Lamina PDF uses it once to open the file on this device and doesn't save it.",
  "reader.password.wrong":
    "That password didn't open the file. Passwords are case-sensitive; check Caps Lock and try again.",
  "reader.password.cancel": "Cancel",
  "reader.password.close": "Close",
  "reader.password.open": "Open",
  "reader.password.opening": "Opening…",
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
