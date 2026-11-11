// The edit session's commands (Paper O01, design language §8.8): Undo and Redo with their
// shortcuts, Show history and Go to step, and turning the current page, the one edit the reader
// makes until Organise (E2) brings the page grid. Undo, Redo and Go to step drive the session in
// the context; Show history and the page turns are the shell's, which knows the rail and the pages.
// Each has a visible home: the top bar's Undo and Redo, the web's More menu (the desktop's Edit
// menu, E1h), the History tab.

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
  { mac: "Mod+Shift+Z", other: "Mod+Shift+Z", scope: "global", fields: false },
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
      enabled: withSession((s) => (stepToUndo(s) === undefined ? message("command.reason.nothingToUndo") : true)),
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
      enabled: withSession((s) => (stepToRedo(s) === undefined ? message("command.reason.nothingToRedo") : true)),
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
    rotateCommand("pages.rotate-right", "rotate-right"),
    rotateCommand("pages.rotate-left", "rotate-left"),
  ];
}

/** Registers the edit session's commands in `registry`; returns the function that removes them. */
export function registerEditCommands(registry: CommandRegistry): () => void {
  const removers = editCommands().map((command) => registry.register(command));
  return () => {
    for (const remove of removers) remove();
  };
}
