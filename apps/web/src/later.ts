// What the reader loads once a document is open, not at start (#380), in one chunk: the print,
// export, properties and initial view dialogs (the work an export does, and the properties form's edits), the Threads and Attachments tabs' panels, the capability page with its matrix, the link previews, reader mode's column and its settings, the object inspector, the History tab, the link editor (links.tsx), the question about several files opened together (several.tsx) and the rest of the
// edit session (the top bar's save state, Undo and Redo, and editing-runtime.tsx), the Comment mode (comment/mode.tsx) and the Edit mode (content/mode.tsx). One chunk, because every part shares the chrome's own
// modules: loaded separately, the bundler would split those shared modules out of the startup
// script into chunks of their own, which compress worse together than as one.

export {
  ExportDialog,
  ExportProgressDialog,
  ImagesToPdfDialog,
  PrintDialog,
  PrintProgressDialog,
} from "@laminapdf/ui/dialogs";
export { ReaderSettingsPanel, ReaderView } from "@laminapdf/ui/reader-mode";
export { AttachmentsTab } from "./attachments-tab.tsx";
export { OutlineEditing } from "./bookmarks.tsx";
export { CapabilityRoutePage } from "./capability-page.tsx";
export { CommentInspector, CommentPanel, commentActions } from "./comment/mode.tsx";
export { ContentInspectorView, contentActions } from "./content/mode.tsx";
export { editMenu } from "./edit-menu.ts";
export { EditArea } from "./edit-tools.tsx";
export { EditingRuntime, editActions, historyTab, SaveNotices } from "./editing-runtime.tsx";
export { startExport } from "./export-run.ts";
export { FILL_SIGN, FormMode, FormStatus, fieldsTab, formActions } from "./forms/mode.tsx";
export { InspectorView } from "./inspector.tsx";
export { LinkPreviews } from "./link-preview.tsx";
export { ModeArea, OrganiseController, organiseActions } from "./organise.tsx";
export { EditablePropertiesDialog } from "./properties-editing.tsx";
export { SaveFlow } from "./save-flow.ts";
export { askAboutSeveral } from "./several.tsx";
export { ThreadsTab } from "./threads-tab.tsx";
