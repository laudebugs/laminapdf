// The dialogs a reader needs only once a document is open and the reader asks for them: Print…
// and Export… with their progress, Document properties, which edits the document's description
// when it can, Initial view… and the new PDF from images; the Threads tab's panel, shown only for a file with article threads,
// the Attachments tab's panel with its warning before a program is saved, and the object
// inspector, a full-canvas view of the file's objects, fonts, colour and claims.
// They are `@laminapdf/ui/dialogs`, an entry of their own, so an app can load them with import()
// after its first paint instead of at start (#380).

export {
  claimsText,
  type InspectColour,
  type InspectConformance,
  type InspectFont,
  type InspectRevision,
} from "./inspector/facts.js";
export {
  type InspectOverview,
  type InspectTab,
  ObjectInspector,
  type ObjectInspectorProps,
} from "./inspector/object-inspector.js";
export { STREAM_CAP, type StreamMode, type StreamState } from "./inspector/stream-block.js";
export type { InspectObjectData, InspectObjectEntry, InspectValue } from "./inspector/values.js";
export {
  type ImagesToPdfChoice,
  ImagesToPdfDialog,
  type ImagesToPdfDialogProps,
} from "./organise/images-to-pdf.js";
export {
  type AttachmentData,
  type AttachmentHolderData,
  type AttachmentRelationshipData,
  attachmentName,
  isProgram,
  mimeOf,
} from "./reader/attachments.js";
export {
  type AttachmentChange,
  AttachmentsPanel,
  type AttachmentsPanelProps,
  type ChecksumState,
  ProgramWarningDialog,
  type ProgramWarningDialogProps,
} from "./reader/attachments-panel.js";
export {
  type ExportChoice,
  type ExportCounts,
  ExportDialog,
  type ExportDialogProps,
  type ExportKind,
} from "./reader/export-dialog.js";
export {
  type ExportDestination,
  ExportProgressDialog,
  type ExportProgressDialogProps,
  exportProgressTitle,
} from "./reader/export-progress.js";
export { InitialViewDialog, type InitialViewDialogProps, initialViewStep } from "./reader/initial-view-dialog.js";
export type {
  InitialLayout,
  InitialMode,
  InitialViewChange,
  InitialViewData,
  InitialZoom,
} from "./reader/initial-view-form.js";
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
export { pdfDateInstant, pdfDateOf } from "./reader/properties-form.js";
export { type PropertiesOp, propertiesOps } from "./reader/properties-ops.js";
export { ThreadsPanel, type ThreadsPanelProps, threadTitle } from "./reader/threads-panel.js";
export type { XmpPacketData } from "./reader/xmp-editor.js";
export type { XmpChange, XmpValueData } from "./reader/xmp-model.js";
