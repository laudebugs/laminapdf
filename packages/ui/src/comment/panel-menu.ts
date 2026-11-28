// The Comments panel's More menu (C06, C07): export, import, flatten and the summary, each with
// the reason it can't run now, read from the command registry so the palette and the menu agree.

import { type CommandView, useCommand } from "../commands/react.js";
import type { MenuEntry } from "../overlays/popover.js";

/** The More menu's items, and what choosing one runs. */
export interface InterchangeMenu {
  readonly items: readonly MenuEntry<string>[];
  /** Runs the command whose id is `id`. */
  readonly run: (id: string) => void;
}

/** A menu item for `view`; none when the command is hidden in this shell. */
function entry(view: CommandView): MenuEntry<string>[] {
  const { availability } = view;
  if (availability.state === "hidden") return [];
  return [
    {
      id: view.id,
      label: view.title,
      ...(view.shortcut === undefined ? {} : { shortcut: view.shortcut }),
      ...(availability.state === "disabled" ? { isDisabled: true, disabledReason: availability.reason } : {}),
    },
  ];
}

/** The More menu's items for the document open, kept current as the reader and session change. */
export function useInterchangeMenu(): InterchangeMenu {
  const views = [
    useCommand("comments.export"),
    useCommand("comments.export-markdown"),
    useCommand("comments.import"),
    useCommand("comments.flatten"),
    useCommand("comments.summary"),
    useCommand("comments.print-summary"),
  ];
  return {
    items: views.flatMap(entry),
    run: (id) => views.find((view) => view.id === id)?.run(),
  };
}
