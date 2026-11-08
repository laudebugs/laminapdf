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
  "reader.page.unknown": "/ –",
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
  "reader.password.open": "Open",
  "reader.password.opening": "Opening…",
  "reader.open.heading": "Open a PDF",
  "reader.open.drop": "Drop a PDF anywhere in this window",
  "reader.open.paste": "or paste one with",
  "reader.open.choose": "Choose a file",
  "reader.open.privacy": "Lamina PDF reads PDFs inside this browser tab and has no upload server.",
  "reader.rotate": "Rotate clockwise",
  "reader.open.dropping": "Drop to open the PDF",
  "reader.open.notPdf": "That isn't a PDF. Lamina PDF opens PDF files only.",
  "reader.open.unreadable":
    "That file couldn't be read. It may have moved, or this browser may have lost access to it.",
  "repair.title": "Repaired on open",
  "repair.details": "Details",
  "repair.close": "Close repair details",
  "repair.notice.title": "This file is damaged.",
  "repair.notice.body":
    "Lamina PDF repaired it in memory so you can read it. The file on your device hasn't changed, and other readers may show it differently.",
  "repair.done": "What Lamina PDF did",
  "repair.notRun": "Not run",
  "repair.showAll": "Show all {count}",
  "repair.copy": "Copy details",
  "repair.pages": "pages {pages}",
  "repairCode.xref-repaired": "Rebuilt the table that locates each object in the file",
  "repairCode.missing-header": "Read the file although its PDF header was missing",
  "repairCode.xrefstm-ignored": "Ignored a broken table of objects and used a rebuilt one",
  "repairCode.stream-length-recovered": "Measured page data whose stated length was wrong",
  "repairCode.stale-generation": "Used objects whose version numbers didn't match their references",
  "repairCode.page-tree-node-skipped": "Left out part of the list of pages that couldn't be read",
  "repairCode.page-count-mismatch": "Counted the pages again, since the stated count was wrong",
  "repairCode.flate-truncated": "Read as much as possible of compressed data that ended early",
  "repairCode.objstm-index-mismatch": "Read objects from a container whose index was wrong",
  "repairCode.object-number-out-of-range": "Ignored references to objects the file doesn't have",
  "repairCode.aes-padding-invalid": "Decrypted data whose padding was wrong",
  "repairCode.perms-mismatch": "Found permissions that didn't match their encrypted copy",
  "repairCode.warnings-truncated": "Stopped listing problems after the first many",
  "repairCode.invalid-box": "Used a standard page size where a page's size couldn't be read",
  "repairCode.invalid-rotate": "Ignored a page rotation that wasn't a quarter turn",
  "repairCode.name-tree-node-skipped": "Left out named destinations or files that couldn't be read",
  "repairCode.outline-cycle": "Stopped following an outline that loops back on itself",
  "repairCode.outline-item-skipped": "Left out outline entries that point to no page",
  "repairCode.invalid-date": "Ignored dates that couldn't be read",
  "repairCode.invalid-page-label": "Ignored page labels that couldn't be read",
  "repairCode.invalid-entry": "Ignored settings that had the wrong kind of value",
  "repairCode.content": "Skipped drawing instructions on a page that couldn't be read",
  "repairCode.font-invalid": "Couldn't read a font, so some text may look different",
  "repairCode.cmap-invalid": "Couldn't read how a font maps its characters, so some text may not copy",
  "repairCode.unmapped-glyph": "Found characters that can't be copied as text",
  "repairCode.extract": "Simplified the page's text where its layout was too complex to follow",
  "repairCode.other": "Repaired a problem in the file",
  "failure.title": "This file can't be opened",
  "failureCode.not-pdf": "This file isn't a PDF.",
  "failureCode.missing-root":
    "Lamina PDF couldn't find the document's list of pages, even after trying to repair it. It may be incomplete: ask the sender for a new copy, or download it again.",
  "failureCode.limit-exceeded": "This file is larger or more complex than Lamina PDF opens safely in a browser tab.",
  "failureCode.unsupported-security-handler": "This file uses a kind of encryption Lamina PDF can't open yet.",
  "failureCode.other":
    "Lamina PDF couldn't read this file, even after trying to repair it. It may be damaged: ask the sender for a new copy, or download it again.",
  "failureCode.retry": "Lamina PDF couldn't finish opening this file. Try again, or reload the page.",
  "failure.tryAnother": "Try another file",
  "failure.copy": "Copy error details",
  "loading.title": "Downloading the whole file",
  "loading.why": "This site sends the file only in one piece, so the pages show once all of it has arrived.",
  "loading.progress": "{loaded} of {total} MB",
  "loading.progressUnknown": "{loaded} MB",
  "loading.cancel": "Cancel",
  "debug.menu": "Debug",
  "debug.backend": "Render with {backend}",
  "debug.backendCurrent": "Render with {backend} (in use)",
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
