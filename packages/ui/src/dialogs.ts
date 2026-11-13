// The dialogs a reader needs only once a document is open and the reader asks for them: Print…
// with its progress, and Document properties, which edits the document's description when it can. They are `@laminapdf/ui/dialogs`, an entry of their
// own, so an app can load them with import() after its first paint instead of at start (#380).

export {
  type PrintChoice,
  PrintDialog,
  type PrintDialogProps,
  type PrintPermissions,
  PrintProgressDialog,
  type PrintProgressDialogProps,
} from "./reader/print-dialog.js";
export {
  type MetadataState,
  PropertiesDialog,
  type PropertiesDialogProps,
  type PropertiesEditing,
  type PropertiesState,
} from "./reader/properties-dialog.js";
export type { DocumentMetadataData, MetadataChange } from "./reader/properties-form.js";
export type { XmpPacketData } from "./reader/xmp-editor.js";
export type { XmpChange, XmpValueData } from "./reader/xmp-model.js";
