// The More menu's edit items (the web's Edit menu, O01), made from the registered edit commands.
// They load with the commands, once a document is open (later.ts, #380): before that there are none.

import type { CommandContext, CommandId, CommandRegistry, GatedEntry } from "@laminapdf/ui";
import { chipKeys } from "@laminapdf/ui";

/** The More menu's edit items (the web's Edit menu, O01), and the command each runs. */
const EDIT_MENU = {
  undo: "edit.undo",
  redo: "edit.redo",
  history: "edit.history",
  "rotate-right": "pages.rotate-right",
  "rotate-left": "pages.rotate-left",
} as const satisfies Record<string, CommandId>;

/** An edit item of the More menu. */
export type EditMenuId = keyof typeof EDIT_MENU;

/**
 * The More menu's edit items, from their registered commands (ADR 0024 §3): the title, the
 * shortcut and whether each can run now; and how to run one.
 */
export function editMenu(
  registry: CommandRegistry,
  context: CommandContext,
  /**
   * What the items are read from, besides the context: the registry's version and the session's
   * state. Named here so the React Compiler makes the items again when either changes.
   */
  _readFrom: { readonly version: number; readonly state: unknown },
): { readonly items: GatedEntry<EditMenuId>[]; readonly run: (id: EditMenuId) => void } {
  // Through the registry, so a command that can't run now doesn't; a failure is the session's to say.
  const run = (id: EditMenuId) => void registry.run(EDIT_MENU[id], context).catch(() => undefined);
  const items = (Object.entries(EDIT_MENU) as [EditMenuId, CommandId][]).flatMap(([id, commandId]) => {
    const command = registry.get(commandId);
    const availability = registry.availability(commandId, context);
    if (command === undefined || availability.state === "hidden") return [];
    const shortcut = chipKeys(command, context);
    return [
      {
        id,
        label: registry.title(command, context),
        ...(command.icon === undefined ? {} : { icon: command.icon }),
        ...(shortcut === undefined ? {} : { shortcut }),
        ...(availability.state === "disabled" ? { isDisabled: true, disabledReason: availability.reason } : {}),
      },
    ];
  });
  return { items, run };
}
