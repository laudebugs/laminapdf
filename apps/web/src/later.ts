// What the reader loads once a document is open, not at start (#380), in one chunk: the print and
// properties dialogs, the capability page with its matrix, the link previews, reader mode's column and its settings, the History tab and the rest of the
// edit session (the top bar's save state, Undo and Redo, and editing-runtime.tsx). One chunk, because every part shares the chrome's own
// modules: loaded separately, the bundler would split those shared modules out of the startup
// script into chunks of their own, which compress worse together than as one.

export { PrintDialog, PrintProgressDialog, PropertiesDialog } from "@laminapdf/ui/dialogs";
export { EditArea } from "@laminapdf/ui/editing";
export { ReaderSettingsPanel, ReaderView } from "@laminapdf/ui/reader-mode";
export { CapabilityRoutePage } from "./capability-page.tsx";
export { editMenu } from "./edit-menu.ts";
export { EditingRuntime, editActions, historyTab, SaveNotices } from "./editing-runtime.tsx";
export { LinkPreviews } from "./link-preview.tsx";
export { SaveFlow } from "./save-flow.ts";
