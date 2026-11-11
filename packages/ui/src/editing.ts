// What an editor needs only once a document is open: the History tab, the edit commands and the
// interaction modes' registry. They are `@laminapdf/ui/editing`, an entry of their own, so an app
// can load them with import() when a document opens instead of at start (#380), as it does
// `./dialogs`. Their types, and the edit session's words the top bar shows, are in the root entry.

export { editCommands, registerEditCommands } from "./commands/edit-commands.js";
export { EditArea } from "./edit/edit-area.js";
export { HistoryPanel, type HistoryPanelProps } from "./edit/history-panel.js";
export { blockedReason, revertedText } from "./edit/outcome.js";
export { attachTools, handTool, SELECT_TOOL, TEXT_SELECT_TOOL, ToolRegistry } from "./tools/tools.js";
