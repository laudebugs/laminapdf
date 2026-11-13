// The native menus as data: `MENU_LAYOUT` with each command's title and shortcut read
// from the command registry. `test/menus-json.test.ts` writes it to `src-tauri/menus.json`, which the main
// process embeds and builds the menus from (src-tauri/src/menu.rs), and fails while the file
// there is out of date.

import {
  type AnyCommand,
  type CommandContext,
  CommandRegistry,
  type KeyPlatform,
  menuAccelerator,
  registerDesktopCommands,
  registerReaderCommands,
} from "@laminapdf/ui";
import { LINKS, MENU_LAYOUT, MENU_WORDS, type MenuEntry, type MenuLayout } from "./layout.ts";

/** A menu item, as `menus.json` holds it. `on` limits it to one platform. */
export type MenuItemJson = (
  | {
      readonly kind: "command";
      readonly id: string;
      readonly title: string;
      /** Its keys on macOS and on Windows and Linux, in the registry's notation (`Mod+Shift+N`). */
      readonly mac?: string;
      readonly other?: string;
      readonly check?: true;
    }
  | {
      readonly kind: "choices";
      readonly id: string;
      readonly title: string;
      readonly choices: readonly { readonly value: string; readonly title: string }[];
    }
  | { readonly kind: "predefined"; readonly name: string }
  | { readonly kind: "link"; readonly id: string; readonly title: string; readonly url: string }
  | { readonly kind: "placeholder"; readonly id: string; readonly title: string }
  | { readonly kind: "recent"; readonly title: string; readonly empty: string; readonly clear: string }
  | { readonly kind: "move-to-window"; readonly title: string; readonly empty: string }
  | { readonly kind: "windows" }
  | { readonly kind: "separator" }
) & { readonly on?: KeyPlatform };

/** A top-level menu, as `menus.json` holds it. */
export interface MenuJson {
  readonly id: MenuLayout["id"];
  readonly title: string;
  readonly on?: KeyPlatform;
  readonly items: readonly MenuItemJson[];
}

/** `menus.json`. */
export interface MenusJson {
  readonly $comment: string;
  readonly menus: readonly MenuJson[];
}

/** A registry with every command the desktop app registers (`app.tsx` registers the same). */
export function desktopRegistry(): CommandRegistry {
  const registry = new CommandRegistry();
  registerReaderCommands(registry);
  registerDesktopCommands(registry);
  return registry;
}

/** The registry and the context the menus read commands in. */
interface Source {
  readonly registry: CommandRegistry;
  readonly ctx: CommandContext;
}

/** Command `id`. @throws Error when the registry lacks it. */
function commandOf({ registry }: Source, id: string): AnyCommand {
  const found = registry.list().find((c) => c.id === id);
  if (found === undefined) throw new Error(`the menus name ${id}, which isn't a command`);
  return found;
}

type Entry<Key extends string> = Extract<MenuEntry, Record<Key, unknown>>;

/** A command's item: its title (with "…" when it opens a dialog) and its keys. */
function commandItem(source: Source, entry: Entry<"command">): MenuItemJson {
  const found = commandOf(source, entry.command);
  const title = source.registry.title(found, source.ctx);
  const mac = menuAccelerator(found, source.ctx, "mac");
  const other = menuAccelerator(found, source.ctx, "other");
  return {
    kind: "command",
    id: found.id,
    title: entry.dialog === true ? withEllipsis(title) : title,
    ...(mac === undefined ? {} : { mac }),
    ...(other === undefined ? {} : { other }),
    ...(entry.check === true ? { check: true } : {}),
  };
}

/** A command's choices, as a submenu. @throws Error when it takes no fixed choices. */
function choicesItem(source: Source, entry: Entry<"choices">): MenuItemJson {
  const found = commandOf(source, entry.choices);
  const choices = found.args?.choices;
  if (choices === undefined) throw new Error(`${entry.choices} takes no fixed choices`);
  return { kind: "choices", id: found.id, title: source.registry.title(found, source.ctx), choices };
}

/** An item that isn't a command's: the OS's own, a link, a placeholder, or a list Rust fills. */
function otherItem(entry: Exclude<MenuEntry, Entry<"command"> | Entry<"choices">>): MenuItemJson {
  if ("predefined" in entry) return { kind: "predefined", name: entry.predefined };
  if ("link" in entry)
    return { kind: "link", id: entry.link, title: MENU_WORDS.link[entry.link], url: LINKS[entry.link] };
  if ("placeholder" in entry) {
    return { kind: "placeholder", id: entry.placeholder, title: MENU_WORDS.placeholder[entry.placeholder] };
  }
  if ("recent" in entry) {
    return { kind: "recent", title: MENU_WORDS.recent, empty: MENU_WORDS.recentEmpty, clear: MENU_WORDS.clearRecent };
  }
  if ("moveToWindow" in entry) {
    return { kind: "move-to-window", title: MENU_WORDS.moveToWindow, empty: MENU_WORDS.noOtherWindows };
  }
  return "windows" in entry ? { kind: "windows" } : { kind: "separator" };
}

/** One entry of the layout as data, with its platform. */
function itemOf(source: Source, entry: MenuEntry): MenuItemJson {
  const on = entry.on === undefined ? {} : { on: entry.on };
  if ("command" in entry) return { ...commandItem(source, entry), ...on };
  if ("choices" in entry) return { ...choicesItem(source, entry), ...on };
  return { ...otherItem(entry), ...on };
}

/**
 * The menus as data: `MENU_LAYOUT`, each command's title and shortcuts from `registry` as they
 * read in `ctx` (the desktop's).
 * @throws Error when the layout names a command the registry lacks, or offers choices of a
 *   command that takes none.
 */
export function nativeMenus(registry: CommandRegistry, ctx: CommandContext): MenusJson {
  const source = { registry, ctx };
  return {
    $comment:
      "Generated from apps/desktop/src/menus/layout.ts and the command registry by `pnpm --filter @laminapdf/desktop menus`. Don't edit: test/menus-json.test.ts fails while this file is out of date.",
    menus: MENU_LAYOUT.map((menu) => ({
      id: menu.id,
      title: MENU_WORDS.menu[menu.id],
      ...(menu.on === undefined ? {} : { on: menu.on }),
      items: menu.entries.map((entry) => itemOf(source, entry)),
    })),
  };
}

/** `title` ending in "…" once. */
const withEllipsis = (title: string): string => (title.endsWith("…") ? title : `${title}…`);

/** The ids of the registry commands the menus list, plain or as choices: what `menu_state` reports. */
export function menuCommandIds(menus: MenusJson): readonly string[] {
  const ids = new Set<string>();
  for (const menu of menus.menus) {
    for (const entry of menu.items) {
      if (entry.kind === "command" || entry.kind === "choices") ids.add(entry.id);
    }
  }
  return [...ids];
}
