// The native menus' layout (Paper S03): which commands each menu lists, in order. It
// names commands by id only: their titles and shortcuts come from the command registry, so the
// menus, the palette and the shortcut help can't disagree. `menus.ts` turns this layout and the
// registry into `src-tauri/menus.json`, which the main process builds the menus from.

import type { CommandId, KeyPlatform } from "@laminapdf/ui";

/** A menu item the OS provides (its title, its keys and what it does are the OS's). */
export type Predefined =
  | "about"
  | "services"
  | "hide"
  | "hide-others"
  | "show-all"
  | "quit"
  | "copy"
  | "select-all"
  | "minimize"
  | "zoom"
  | "full-screen"
  | "bring-all-to-front";

/** A page the Help menu opens in the browser. */
export type Link = "help" | "issues";

/** An item whose work comes in a later todo: shown, never enabled. */
export type Placeholder = "check-for-updates" | "licences";

/** One entry of a menu. Each may exist on one platform only (`on`). */
export type MenuEntry = (
  | {
      readonly command: CommandId;
      /** A toggle: a check item, ticked while it is on. */
      readonly check?: true;
      /** It opens a dialog first: its title ends in "…", as the platforms' menus write it. */
      readonly dialog?: true;
    }
  | { readonly choices: CommandId }
  | { readonly predefined: Predefined }
  | { readonly link: Link }
  | { readonly placeholder: Placeholder }
  | { readonly recent: true }
  | { readonly moveToWindow: true }
  | { readonly windows: true }
  | { readonly separator: true }
) & { readonly on?: KeyPlatform };

/** A top-level menu. */
export interface MenuLayout {
  readonly id: "app" | "file" | "edit" | "view" | "go" | "tools" | "window" | "help";
  readonly on?: KeyPlatform;
  readonly entries: readonly MenuEntry[];
}

const SEPARATOR = { separator: true } as const;
const MAC = "mac" as const;
const OTHER = "other" as const;

/** The menus, left to right. macOS adds its app menu first; Windows and Linux have no app menu. */
export const MENU_LAYOUT: readonly MenuLayout[] = [
  {
    id: "app",
    on: MAC,
    entries: [
      { predefined: "about" },
      { placeholder: "check-for-updates" },
      SEPARATOR,
      { command: "app.settings", dialog: true },
      SEPARATOR,
      { predefined: "services" },
      SEPARATOR,
      { predefined: "hide" },
      { predefined: "hide-others" },
      { predefined: "show-all" },
      SEPARATOR,
      { predefined: "quit" },
    ],
  },
  {
    id: "file",
    entries: [
      { command: "file.open", dialog: true },
      { recent: true },
      SEPARATOR,
      { command: "window.new" },
      { command: "tab.close" },
      { command: "window.close" },
      SEPARATOR,
      { command: "file.save-copy" },
      { command: "file.properties", dialog: true },
      SEPARATOR,
      { command: "file.print", dialog: true },
      { separator: true, on: OTHER },
      { command: "app.quit", on: OTHER },
    ],
  },
  {
    id: "edit",
    entries: [
      { predefined: "copy" },
      { predefined: "select-all" },
      SEPARATOR,
      { command: "find.open" },
      { command: "find.next" },
      { command: "find.previous" },
      { separator: true, on: OTHER },
      { command: "app.settings", dialog: true, on: OTHER },
    ],
  },
  {
    id: "view",
    entries: [
      { command: "view.zoom-in" },
      { command: "view.zoom-out" },
      { command: "view.actual-size" },
      { command: "view.fit-width" },
      { command: "view.fit-page" },
      SEPARATOR,
      { command: "view.toggle-sidebar", check: true },
      { command: "view.focus-mode", check: true },
      { command: "view.rotate-display" },
      SEPARATOR,
      { choices: "view.page-layout" },
      { choices: "view.page-appearance" },
      { choices: "view.theme" },
      { choices: "view.accent" },
      { separator: true, on: MAC },
      { predefined: "full-screen", on: MAC },
    ],
  },
  {
    id: "go",
    entries: [
      { command: "nav.next-page" },
      { command: "nav.previous-page" },
      { command: "nav.first-page" },
      { command: "nav.last-page" },
      { command: "nav.go-to-page" },
      SEPARATOR,
      { command: "nav.back" },
      { command: "nav.forward" },
    ],
  },
  {
    id: "tools",
    entries: [{ command: "palette.open" }, { command: "view.reader-mode" }, { command: "privacy.network-activity" }],
  },
  {
    id: "window",
    entries: [
      { predefined: "minimize" },
      { predefined: "zoom", on: MAC },
      SEPARATOR,
      { command: "tab.next" },
      { command: "tab.previous" },
      { command: "tab.move-to-new-window" },
      { moveToWindow: true },
      SEPARATOR,
      { predefined: "bring-all-to-front", on: MAC },
      { windows: true },
    ],
  },
  {
    id: "help",
    entries: [
      { link: "help" },
      { command: "help.shortcuts" },
      { command: "help.capabilities" },
      SEPARATOR,
      { link: "issues" },
      { placeholder: "licences" },
      { separator: true, on: OTHER },
      { placeholder: "check-for-updates", on: OTHER },
      { predefined: "about", on: OTHER },
    ],
  },
];

/**
 * The words the menus use besides the commands' titles: the menus' names and the items that
 * aren't commands. English for now, kept in one table for the translations to come.
 */
export const MENU_WORDS = {
  menu: {
    app: "Lamina PDF",
    file: "File",
    edit: "Edit",
    view: "View",
    go: "Go",
    tools: "Tools",
    window: "Window",
    help: "Help",
  },
  recent: "Open recent",
  recentEmpty: "No recent files",
  clearRecent: "Clear menu",
  moveToWindow: "Move tab to window",
  noOtherWindows: "No other windows",
  link: { help: "Lamina PDF help", issues: "Report an issue" },
  placeholder: { "check-for-updates": "Check for updates…", licences: "View licences" },
} as const;

/** Where the Help menu's links go. */
export const LINKS: Readonly<Record<Link, string>> = {
  help: "https://docs.laminapdf.com/",
  issues: "https://github.com/laudebugs/laminapdf/issues/new/choose",
};
