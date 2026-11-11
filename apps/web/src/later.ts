// What the reader loads once a document is open, not at start (#380), in one chunk: the print and
// properties dialogs, reader mode's column and its settings, the History tab and the rest of the
// edit session (the top bar's save state, Undo and Redo, and editing-runtime.tsx). One chunk, because every part shares the chrome's own
// modules: loaded separately, the bundler would split those shared modules out of the startup
// script into chunks of their own, which compress worse together than as one.

export { PrintDialog, PrintProgressDialog, PropertiesDialog } from "@laminapdf/ui/dialogs";
export { EditArea, HistoryPanel } from "@laminapdf/ui/editing";
export { ReaderSettingsPanel, ReaderView } from "@laminapdf/ui/reader-mode";
export { EditingRuntime } from "./editing-runtime.tsx";
