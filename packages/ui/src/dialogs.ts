// The dialogs a reader needs only once a document is open and the reader asks for them: Print…
// with its progress, and Document properties. They are `@laminapdf/ui/dialogs`, an entry of their
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
  PropertiesDialog,
  type PropertiesDialogProps,
  type PropertiesState,
} from "./reader/properties-dialog.js";
