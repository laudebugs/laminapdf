// The editor's part of the top bar's file area (§7.1, Paper O01): the save state after the file
// name, then a divider and Undo and Redo. \`ReaderTopBar\` takes it as \`editArea\`; it is in
// \`@laminapdf/ui/editing\`, which an app loads once a document is open (#380). Undo and Redo run
// \`edit.undo\` and \`edit.redo\`, so it sits inside a \`CommandsProvider\`.

import type { ReactNode } from "react";
import { useCommand, useCommands } from "../commands/react.js";
import { isBound, onPlatform } from "../commands/shortcuts.js";
import type { CommandContext, CommandId, Shortcut } from "../commands/types.js";
import { IconButton } from "../components/button.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { useStoreState } from "../state/hooks.js";
import { type EditControls, saveStateText, sessionStarted } from "./edit-controls.js";

/** The editor's part of the file area: the save state, then Undo and Redo; nothing in the reader. */
export function EditArea({ edits, keys }: { edits: EditControls; keys: KeyPlatform }): ReactNode {
  return (
    <>
      <SaveState edits={edits} />
      <UndoRedo keys={keys} />
    </>
  );
}

/**
 * The save state (O01): "No unsaved changes", "4 unsaved changes", announced politely as it
 * changes; shown once the document has an edit session.
 */
function SaveState({ edits }: { edits: EditControls }): ReactNode {
  const started = useStoreState(edits, sessionStarted);
  const text = useStoreState(edits, saveStateText);
  return (
    <span className="paper-top-bar-save-state" aria-live="polite">
      {started ? text : ""}
    </span>
  );
}

/** Undo and Redo (O01): named by the step they take back or make again, disabled with none. */
function UndoRedo({ keys }: { keys: KeyPlatform }): ReactNode {
  const undo = useCommand("edit.undo");
  const redo = useCommand("edit.redo");
  const { registry, context } = useCommands();
  if (undo.availability.state === "hidden" && redo.availability.state === "hidden") return null;
  const shortcut = (id: CommandId) => keyShortcuts(registry.get(id)?.shortcuts ?? [], context, keys);
  return (
    // biome-ignore lint/a11y/useSemanticElements: a toolbar group of buttons, as History's is; <fieldset> holds form controls.
    <div className="paper-top-bar-edit" role="group" aria-label={message("commandGroup.edit")}>
      {/* No hover tooltip yet: the top bar's other buttons have none either, and the tooltip's code
          would weigh on the startup script (#380); the names carry the step and the keys. */}
      {[undo, redo].map((view) => (
        <IconButton
          key={view.id}
          icon={view.id === "edit.undo" ? "undo" : "redo"}
          label={view.title}
          aria-keyshortcuts={shortcut(view.id)}
          isDisabled={view.availability.state !== "available"}
          onPress={() => view.run()}
        />
      ))}
    </div>
  );
}

/**
 * `aria-keyshortcuts` for `bindings` on `platform`: each bound one's keys in ARIA's names, space
 * apart ("Control+Y Control+Shift+Z"); undefined with none.
 */
export function keyShortcuts(
  bindings: readonly Shortcut[],
  ctx: CommandContext,
  platform: KeyPlatform,
): string | undefined {
  const keys = bindings
    .filter((b) => isBound(b, ctx) && onPlatform(b, platform))
    .map((b) => b[platform].replace("Mod", platform === "mac" ? "Meta" : "Control"));
  return keys.length === 0 ? undefined : [...new Set(keys)].join(" ");
}
