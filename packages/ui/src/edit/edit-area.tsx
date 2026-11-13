// The editor's part of the top bar's file area (§7.1, Paper O01, O02): the save state after the file
// name, then a divider, Undo and Redo, and Save with its options while there are changes. \`ReaderTopBar\` takes it as \`editArea\`; it is in
// \`@laminapdf/ui/editing\`, which an app loads once a document is open (#380). Undo and Redo run
// \`edit.undo\` and \`edit.redo\`, so it sits inside a \`CommandsProvider\`.

import type { ReactNode } from "react";
import { type CommandView, useCommand, useCommands } from "../commands/react.js";
import { isBound, onPlatform } from "../commands/shortcuts.js";
import type { CommandContext, CommandId, Shortcut } from "../commands/types.js";
import { IconButton } from "../components/button.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { ToolToggle } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";
import { message } from "../messages.js";
import { Tooltip } from "../overlays/tooltip.js";
import { useStoreState } from "../state/hooks.js";
import type { ToolRegistry } from "../tools/tools.js";
import { type EditControls, saveStateText, sessionStarted } from "./edit-controls.js";
import { SaveButton } from "./save-ui.js";

/**
 * The editor's part of the file area: the save state, then Undo and Redo, and the Link tool
 * (O12, while the reader has no Edit mode to hold it) once `tools` has it; nothing in the reader.
 */
export function EditArea({
  edits,
  keys,
  tools,
}: {
  edits: EditControls;
  keys: KeyPlatform;
  tools?: ToolRegistry;
}): ReactNode {
  return (
    <>
      <SaveState edits={edits} />
      <UndoRedo keys={keys} />
      {tools === undefined ? null : <LinkTool tools={tools} />}
      <SaveButton keys={keys} />
    </>
  );
}

/** The Link tool's toggle: pressed while it is on; off goes back to selecting text. */
function LinkTool({ tools }: { tools: ToolRegistry }): ReactNode {
  const registered = useStoreState(tools, (s) => s.tools.has("link"));
  const on = useStoreState(tools, (s) => s.current === "link");
  if (!registered) return null;
  return (
    <ToolToggle icon="link" isSelected={on} onChange={(next) => tools.use(next ? "link" : "select")}>
      {laterMessage("link.tool")}
    </ToolToggle>
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
      {/* The tooltip names the step with its keys (O01); this chunk loads only once a document is
          open, so its code stays out of the startup script (#380). */}
      {[undo, redo].map((view) => (
        <StepButton
          key={view.id}
          view={view}
          keys={keys}
          shortcut={registry.get(view.id)?.shortcuts?.[0]}
          ariaKeys={shortcut(view.id)}
        />
      ))}
    </div>
  );
}

/** Undo or Redo with its tooltip: the step, and the first of its keys as a chip. */
function StepButton({
  view,
  keys,
  shortcut,
  ariaKeys,
}: {
  view: CommandView;
  keys: KeyPlatform;
  shortcut: Shortcut | undefined;
  ariaKeys: string | undefined;
}): ReactNode {
  const button = (
    <IconButton
      icon={view.id === "edit.undo" ? "undo" : "redo"}
      label={view.title}
      aria-keyshortcuts={ariaKeys}
      isDisabled={view.availability.state !== "available"}
      onPress={() => view.run()}
    />
  );
  return shortcut === undefined ? (
    <Tooltip text={view.title}>{button}</Tooltip>
  ) : (
    <Tooltip text={view.title} shortcut={shortcut} platform={keys}>
      {button}
    </Tooltip>
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
