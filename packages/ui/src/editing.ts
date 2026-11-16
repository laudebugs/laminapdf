// What an editor needs only once a document is open: the History tab, the edit commands and the
// interaction modes' registry. They are `@laminapdf/ui/editing`, an entry of their own, so an app
// can load them with import() when a document opens instead of at start (#380), as it does
// `./dialogs`. Their types, and the edit session's words the top bar shows, are in the root entry.

export { editCommands, registerEditCommands } from "./commands/edit-commands.js";
export { type BookmarkMove, dropMove, keyMove, placeOf, type SiblingAt } from "./edit/bookmark-moves.js";
export {
  BookmarkEditor,
  type BookmarkEditorProps,
  type BookmarkItem,
  type BookmarkStep,
  type BookmarkStyle,
  bookmarkWords,
} from "./edit/bookmarks.js";
export { EditArea } from "./edit/edit-area.js";
export { HistoryPanel, type HistoryPanelProps } from "./edit/history-panel.js";
export {
  type AddressCheck,
  LinkEditor,
  type LinkEditorProps,
  type LinkGoal,
  LinkPanel,
  type LinkPanelProps,
  type LinkWord,
  linkWords,
  mailParts,
  mailUri,
} from "./edit/link-editor.js";
export {
  type NamedDestinationRow,
  NamedDestinationsDialog,
  type NamedDestinationsDialogProps,
} from "./edit/named-destinations.js";
export { blockedReason, revertedText } from "./edit/outcome.js";
export {
  ConformanceDialog,
  RemoveSignaturesDialog,
  type SavePlanSlice,
  type SaveSignature,
  SaveStrip,
  signatureKind,
} from "./edit/save-ui.js";
// The words of the parts that load after startup, which only lazily loaded code (this entry's) reads.
export { type LaterMessageId, laterCountMessage, laterMessage } from "./later-messages.js";
export {
  ORGANISE_COMMANDS,
  type OrganiseCommand,
  PAGE_SETUP_COMMANDS,
  registerOrganiseCommands,
} from "./organise/commands.js";
export {
  BlankPagesDialog,
  type BlankSize,
  type BlankWhere,
  MoveToPageDialog,
  movedMessage,
  pageNamed,
  SignedPageDialog,
  selectionNote,
} from "./organise/dialogs.js";
export {
  type FromFileChoice,
  type FromFileWhere,
  PagesFromFileDialog,
  type PagesFromFileDialogProps,
  type SourceFile,
} from "./organise/from-file.js";
export {
  type LengthUnit,
  type Margins,
  type PageSize as SetupPageSize,
  unitFor,
} from "./organise/geometry.js";
export type { LabelRangeValue } from "./organise/labels.js";
export { PageLabelsDialog, type PageLabelsDialogProps } from "./organise/labels-dialog.js";
export {
  type MergeChoice,
  MergeDialog,
  type MergeDialogProps,
  type MergeFile,
  type MergeResult,
  unsharedClaims,
} from "./organise/merge.js";
export { gapOf, type MoveTo, moveToGap, type PageMove, pageRanges, stepGap } from "./organise/moves.js";
export { type GridCommand, type GridPage, PageGrid } from "./organise/page-grid.js";
export {
  type ApplyTo,
  type Box,
  type BoxChanges,
  CropDialog,
  type CropDialogProps,
  PageBoxesDialog,
  type PageBoxesDialogProps,
  type PageBoxValues,
  type ResizeAnchor,
  type ResizeContent,
  ResizeDialog,
  type ResizeDialogProps,
} from "./organise/page-setup.js";
export {
  type Chapter,
  MAX_PARTS,
  type Part,
  type PartNaming,
  type PartsResult,
  partNames,
  partsAtBookmarks,
  partsEvery,
  partsOfRanges,
} from "./organise/parts.js";
export {
  deletedMessage,
  duplicatedMessage,
  insertedMessage,
  insertLabel,
  type OrganiseStep,
  stepLabel,
} from "./organise/report.js";
export { type NamedPart, type SplitBy, SplitDialog, type SplitDialogProps } from "./organise/split.js";
export { OrganiseToolbar, THUMBNAIL_SCALES } from "./organise/toolbar.js";
export { attachTools, handTool, SELECT_TOOL, TEXT_SELECT_TOOL, ToolRegistry } from "./tools/tools.js";
