// The edit session's commands (Paper O01, design language §8.8): Undo and Redo with their
// shortcuts, Show history and Go to step, and turning the current page, the one edit the reader
// makes until Organise (E2) brings the page grid. Undo, Redo and Go to step drive the session in
// the context; Show history and the page turns are the shell's, which knows the rail and the pages.
// Each has a visible home: the top bar's Undo and Redo, the web's More menu (the desktop's Edit
// menu, once it has one), the History tab. Save, Save as… and Save as optimised (Paper O02) are the
// shell's too, which plans the save, asks before one that breaks a signature, and writes the file
// through the platform; their home is the top bar's Save button and its options.

import { type EditControls, type EditSlice, inSentence, stepToRedo, stepToUndo } from "../edit/edit-controls.js";
import { blockedReason } from "../edit/outcome.js";
import { message } from "../messages.js";
import type { CommandRegistry } from "./registry.js";
import type { AnyCommand, ArgsSchema, Command, CommandContext, CommandId, Shortcut } from "./types.js";

/** `true` when the session can take a call now, else why not. */
function editable(ctx: CommandContext): true | string {
  if (ctx.edits === undefined) return message("command.reason.noDocument");
  const state = ctx.edits.getState();
  return blockedReason(state.blocked) ?? true;
}

/** `editable`, then `check` on the session's state. */
function withSession(check: (state: EditSlice) => true | string) {
  return (ctx: CommandContext): true | string => {
    const ok = editable(ctx);
    return ok === true ? check((ctx.edits as EditControls).getState()) : ok;
  };
}

/**
 * Undo's keys: ⌘Z and Ctrl Z. Redo's: ⇧⌘Z on macOS; Ctrl Y and Ctrl Shift Z elsewhere. In a text
 * field the field's own undo keeps them (`fields: false`).
 */
const UNDO_KEYS: readonly Shortcut[] = [{ mac: "Mod+Z", other: "Mod+Z", scope: "global", fields: false }];
const REDO_KEYS: readonly Shortcut[] = [
  { mac: "Mod+Shift+Z", other: "Mod+Y", scope: "global", fields: false },
  { mac: "Mod+Shift+Z", other: "Mod+Shift+Z", scope: "global", fields: false, on: "other" },
];

/** A step number, for `edit.go-to-step`: 0 is the file as opened, n the state after the n-th step kept. */
function stepArgs(): ArgsSchema<number> {
  return {
    prompt: message("command.reason.stepPrompt"),
    prefixes: ["step"],
    parse: (text, ctx) => {
      const total = ctx.edits?.getState().history.steps.length ?? 0;
      const value = text.trim();
      const step = /^\d+$/.test(value) ? Number(value) : Number.NaN;
      if (Number.isInteger(step) && step >= 0 && step <= total) {
        return { ok: true, value: step, summary: message("command.arg.step", { step }) };
      }
      return { ok: false, error: message("command.reason.noStep", { value, total }) };
    },
  };
}

/** Turning the current page, a command the shell runs: it knows the page and its rotation. */
function rotateCommand(id: CommandId, icon: "rotate-left" | "rotate-right"): Command {
  return {
    id,
    title: message(`command.${id}` as Parameters<typeof message>[0]),
    keywords: ["turn", "page", "rotate"],
    group: "edit",
    icon,
    delegated: true,
    home: ["edit-menu"],
    enabled: editable,
    run: (ctx) => ctx.actions?.[id]?.(undefined),
  };
}

/** Initial view… (O13): the shell opens the dialog, which knows the document's pages. */
function initialViewCommand(): Command {
  return {
    id: "organise.initial-view",
    title: message("command.organise.initial-view"),
    keywords: ["open", "opens", "zoom", "layout", "page mode", "viewer preferences"],
    group: "edit",
    delegated: true,
    home: ["edit-menu"],
    enabled: editable,
    run: (ctx) => ctx.actions?.["organise.initial-view"]?.(undefined),
  };
}

/** Save's keys: ⌘S and Ctrl S, in a text field too. Save as…'s: ⇧⌘S and Ctrl Shift S. */
const SAVE_KEYS: readonly Shortcut[] = [{ mac: "Mod+S", other: "Mod+S", scope: "global" }];
const SAVE_AS_KEYS: readonly Shortcut[] = [{ mac: "Mod+Shift+S", other: "Mod+Shift+S", scope: "global" }];

/** `editable`, unless a save is already writing. */
const notSaving = withSession((s) => (s.saving === true ? message("command.reason.saving") : true));

/** A save command the shell runs (`actions`). */
function saveCommand(command: Omit<Command, "group" | "delegated" | "run">): Command {
  const { id } = command;
  return { ...command, group: "file", delegated: true, run: (ctx) => ctx.actions?.[id]?.(undefined) };
}

/** Save, Save as… and Save as optimised. */
function saveCommands(): readonly Command[] {
  return [
    saveCommand({
      id: "file.save",
      // Where the browser can neither write the file nor ask where to save, Save downloads it.
      title: (ctx) => {
        const { canWriteInPlace, canSaveAsDialog } = ctx.platform.capabilities;
        return message(canWriteInPlace || canSaveAsDialog ? "command.file.save" : "command.file.save.download");
      },
      keywords: ["write", "keep", "download"],
      shortcuts: SAVE_KEYS,
      home: ["top-bar"],
      enabled: (ctx) => {
        const ok = notSaving(ctx);
        if (ok !== true) return ok;
        return (ctx.edits as EditControls).getState().history.unsaved > 0
          ? true
          : message("command.reason.nothingToSave");
      },
    }),
    saveCommand({
      id: "file.save-as",
      title: message("command.file.save-as"),
      keywords: ["write", "new file", "export"],
      shortcuts: SAVE_AS_KEYS,
      requires: "canSaveAsDialog",
      home: ["top-bar"],
      enabled: notSaving,
    }),
    saveCommand({
      id: "file.save-optimised",
      title: message("command.file.save-optimised"),
      keywords: ["rewrite", "compact", "smaller"],
      home: ["top-bar"],
      enabled: notSaving,
    }),
  ];
}

/** The edit session's commands, built in the catalogue's words. */
export function editCommands(): readonly AnyCommand[] {
  return [
    {
      id: "edit.undo",
      // The step it takes back, as the button and the menu name it: "Undo rotate page 5".
      title: (ctx) => {
        const step = ctx.edits === undefined ? undefined : stepToUndo(ctx.edits.getState());
        return step === undefined
          ? message("command.edit.undo")
          : message("edit.undo.step", { step: inSentence(step.label) });
      },
      keywords: ["back", "revert"],
      group: "edit",
      icon: "undo",
      shortcuts: UNDO_KEYS,
      home: ["top-bar", "edit-menu"],
      // While a call runs the step isn't known yet: the session checks again when the undo's turn comes.
      enabled: withSession((s) =>
        s.busy || stepToUndo(s) !== undefined ? true : message("command.reason.nothingToUndo"),
      ),
      run: async (ctx) => {
        await ctx.edits?.undo();
      },
    },
    {
      id: "edit.redo",
      title: (ctx) => {
        const step = ctx.edits === undefined ? undefined : stepToRedo(ctx.edits.getState());
        return step === undefined
          ? message("command.edit.redo")
          : message("edit.redo.step", { step: inSentence(step.label) });
      },
      keywords: ["again"],
      group: "edit",
      icon: "redo",
      shortcuts: REDO_KEYS,
      home: ["top-bar", "edit-menu"],
      enabled: withSession((s) =>
        s.busy || stepToRedo(s) !== undefined ? true : message("command.reason.nothingToRedo"),
      ),
      run: async (ctx) => {
        await ctx.edits?.redo();
      },
    },
    {
      id: "edit.history",
      title: message("command.edit.history"),
      keywords: ["undo", "changes", "steps"],
      group: "edit",
      icon: "clock",
      delegated: true,
      home: ["edit-menu", "history-panel"],
      enabled: (ctx) => (ctx.edits === undefined ? message("command.reason.noDocument") : true),
      run: (ctx) => ctx.actions?.["edit.history"]?.(undefined),
    },
    {
      id: "edit.go-to-step",
      title: message("command.edit.go-to-step"),
      keywords: ["history", "undo", "redo"],
      group: "edit",
      home: ["history-panel"],
      args: stepArgs(),
      enabled: editable,
      run: async (ctx, step: number) => {
        await ctx.edits?.goTo(step);
      },
    } satisfies Command<number>,
    initialViewCommand(),
    rotateCommand("pages.rotate-right", "rotate-right"),
    rotateCommand("pages.rotate-left", "rotate-left"),
    ...saveCommands(),
  ];
}

/** Registers the edit session's commands in `registry`; returns the function that removes them. */
export function registerEditCommands(registry: CommandRegistry): () => void {
  const removers = editCommands().map((command) => registry.register(command));
  return () => {
    for (const remove of removers) remove();
  };
}
