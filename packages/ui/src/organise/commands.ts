// The Organise mode's page commands (Paper O06): insert blank pages, delete, duplicate, reverse and
// Move to page…, each on the grid's selection, and Select all; Page setup's (O09, O10): crop,
// resize, page boxes, make landscape or portrait, and page labels; and the commands across files
// (O07, O08): insert pages from a file, a new document from images, replace pages, extract pages, merge and split. The shell runs them (`delegated`): it
// knows the selection and the pages, and asks before one a signature forbids. Their home is the
// Organise toolbar and the page menu; Rotate left and right are the edit commands' (edit-commands.ts),
// which turn the selection while Organise is the mode.

import type { IconName } from "@laminapdf/icons";
import { delegatedCommand } from "../commands/delegated.js";
import type { CommandRegistry } from "../commands/registry.js";
import type { Command } from "../commands/types.js";

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
  "pages.insert-images",
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
  "pages.insert-images",
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
  "pages.insert-images": "image",
  "pages.replace": "replace",
  "pages.extract": "extract",
  "document.merge-add-files": "file",
};

function organiseCommand(id: OrganiseCommand): Command {
  const icon = ICONS[id];
  return delegatedCommand(id, {
    keywords: id.startsWith("document.") ? ["document", "file", "pages", "organise"] : ["page", "pages", "organise"],
    noSession: "organise.unavailable",
    notInMode: "organise.inOrganise",
    icon,
    home: ["organise-toolbar", "context-menu"],
  });
}

/** Registers the Organise mode's page commands in `registry`; returns the function that removes them. */
export function registerOrganiseCommands(registry: CommandRegistry): () => void {
  const removers = ORGANISE_COMMANDS.map((id) => registry.register(organiseCommand(id)));
  return () => {
    for (const remove of removers) remove();
  };
}
