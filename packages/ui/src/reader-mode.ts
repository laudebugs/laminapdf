// Reader mode's column and its settings panel (R14), which a reader needs only once reader mode
// turns on. They are `@laminapdf/ui/reader-mode`, an entry of their own, so an app can load them
// with import() then instead of at start (#380). The settings, the blocks' types and their words
// are in the root entry.

export { isCropBlock, ReaderSettingsPanel, ReaderView } from "./reader/reader-view.js";
