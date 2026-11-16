// The Organise mode's page commands (Paper O06): insert blank pages, delete, duplicate, reverse and
// Move to page…, each on the grid's selection, and Select all; Page setup's (O09, O10): crop,
// resize, page boxes, make landscape or portrait, and page labels; and the commands across files
// (O07, O08): insert pages from a file, replace pages, extract pages, merge and split. The shell runs them (`delegated`): it
// knows the selection and the pages, and asks before one a signature forbids. Their home is the
// Organise toolbar and the page menu; Rotate left and right are the edit commands' (edit-commands.ts),
// which turn the selection while Organise is the mode.

import type { IconName } from "@laminapdf/icons";
import type { CommandRegistry } from "../commands/registry.js";
import type { Command, CommandContext, CommandId } from "../commands/types.js";
import { blockedReason } from "../edit/outcome.js";
import { laterMessage } from "../later-messages.js";

/** The page commands Organise adds. */
export const ORGANISE_COMMANDS = [
  "pages.insert-blank",
  "pages.delete",
  "pages.duplicate",
  "pages.reverse",
  "pages.move-to",
  "pages.select-all",
  "pages.crop",
  "pages.resize",
  "pages.edit-boxes",
  "pages.make-landscape",
  "pages.make-portrait",
  "pages.edit-labels",
  "pages.insert-from-file",
  "pages.replace",
  "pages.extract",
  "document.merge",
  "document.merge-add-files",
  "document.split",
] as const;

/** Insert ▾'s commands (O06, O07), in the menu's order. */
export const INSERT_COMMANDS = [
  "pages.insert-blank",
  "pages.insert-from-file",
  "pages.replace",
] as const satisfies readonly OrganiseCommand[];

/** More page actions' commands (O06, O07, O08), in the menu's order. */
export const MORE_COMMANDS = [
  "pages.duplicate",
  "pages.reverse",
  "pages.select-all",
  "pages.extract",
  "document.split",
  "document.merge",
  "document.merge-add-files",
] as const satisfies readonly OrganiseCommand[];

/** Page setup ▾'s commands (O09, O10), in the menu's order. */
export const PAGE_SETUP_COMMANDS = [
  "pages.crop",
  "pages.resize",
  "pages.edit-boxes",
  "pages.make-landscape",
  "pages.make-portrait",
  "pages.edit-labels",
] as const satisfies readonly OrganiseCommand[];

/** One of them. */
export type OrganiseCommand = (typeof ORGANISE_COMMANDS)[number];

const ICONS: Partial<Record<OrganiseCommand, IconName>> = {
  "pages.insert-blank": "insert-page",
  "pages.delete": "delete",
  "pages.duplicate": "copy",
  "pages.move-to": "move-to-page",
  "pages.crop": "crop",
  "pages.resize": "resize",
  "pages.edit-labels": "tag",
  "pages.insert-from-file": "file",
  "pages.replace": "replace",
  "pages.extract": "extract",
  "document.merge-add-files": "file",
};

/** Whether the session can take an edit now, and the shell gave the command a handler (Organise is open). */
function available(id: CommandId) {
  return (ctx: CommandContext): true | string => {
    if (ctx.edits === undefined) return laterMessage("organise.unavailable");
    const blocked = blockedReason(ctx.edits.getState().blocked);
    if (blocked !== undefined) return blocked;
    return ctx.actions?.[id] === undefined ? laterMessage("organise.inOrganise") : true;
  };
}

function organiseCommand(id: OrganiseCommand): Command {
  const icon = ICONS[id];
  return {
    id,
    title: laterMessage(`command.${id}`),
    keywords: id.startsWith("document.") ? ["document", "file", "pages", "organise"] : ["page", "pages", "organise"],
    group: "edit",
    ...(icon === undefined ? {} : { icon }),
    delegated: true,
    home: ["organise-toolbar", "context-menu"],
    enabled: available(id),
    run: (ctx) => ctx.actions?.[id]?.(undefined),
  };
}

/** Registers the Organise mode's page commands in `registry`; returns the function that removes them. */
export function registerOrganiseCommands(registry: CommandRegistry): () => void {
  const removers = ORGANISE_COMMANDS.map((id) => registry.register(organiseCommand(id)));
  return () => {
    for (const remove of removers) remove();
  };
}
