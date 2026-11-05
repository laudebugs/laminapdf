// The shortcut chip (§7.10): a command's keys as the platform writes them. macOS runs glyphs
// together in its modifier order (⌥⌘E); Windows and Linux spell the modifiers out with spaces
// (Ctrl Shift E). The app says which platform it runs on, so this component never checks.

import type { ReactNode } from "react";

/**
 * A command's keys on each platform, as the command registry declares them (ADR 0024):
 * `"Mod+Shift+E"`, `"?"`, `"F3"`, where `Mod` is ⌘ on macOS and Ctrl elsewhere.
 */
export interface ShortcutKeys {
  readonly mac: string;
  readonly other: string;
}

/** Which way to write the keys. */
export type KeyPlatform = "mac" | "other";

/** Each modifier's names in a key description, its macOS glyph and its word elsewhere. */
const MODIFIERS = {
  ctrl: { names: ["ctrl", "control"], mac: "⌃", other: "Ctrl" },
  alt: { names: ["alt", "option"], mac: "⌥", other: "Alt" },
  shift: { names: ["shift"], mac: "⇧", other: "Shift" },
  mod: { names: ["mod", "cmd", "meta"], mac: "⌘", other: "Ctrl" },
} as const;

/** The order each platform writes modifiers in: macOS puts ⌘ last, Windows puts Ctrl first. */
const ORDER = {
  mac: ["ctrl", "alt", "shift", "mod"],
  other: ["mod", "ctrl", "alt", "shift"],
} as const;

/** Keys whose names read better as a glyph or a short word. */
const KEYS: Record<string, { mac: string; other: string }> = {
  escape: { mac: "esc", other: "Esc" },
  enter: { mac: "↩", other: "Enter" },
  backspace: { mac: "⌫", other: "Backspace" },
  delete: { mac: "⌦", other: "Delete" },
  tab: { mac: "⇥", other: "Tab" },
  arrowup: { mac: "↑", other: "↑" },
  arrowdown: { mac: "↓", other: "↓" },
  arrowleft: { mac: "←", other: "←" },
  arrowright: { mac: "→", other: "→" },
};

/** `"Mod+Shift+E"` written for `platform`: `⇧⌘E` or `Ctrl Shift E`. */
function keysText(keys: string, platform: KeyPlatform): string {
  // Split on each "+" that has something after it, so "Mod++" is Mod and "+".
  const parts = keys.split(/\+(?=.)/);
  const key = parts.pop() ?? "";
  const held = new Set(parts.map((p) => p.toLowerCase()));
  const modifiers = ORDER[platform]
    .filter((m) => MODIFIERS[m].names.some((n) => held.has(n)))
    .map((m) => MODIFIERS[m][platform]);
  const named = KEYS[key.toLowerCase()];
  const last = named === undefined ? (key.length === 1 ? key.toUpperCase() : key) : named[platform];
  // Windows writes Ctrl once, whether it came from Ctrl or Mod.
  const words = [...new Set(modifiers), last];
  return platform === "mac" ? words.join("") : words.join(" ");
}

/** Props of `ShortcutChip`. */
export interface ShortcutChipProps {
  /** The command's keys. */
  keys: ShortcutKeys;
  /** The platform the app runs on, from its `Platform` adapter. */
  platform: KeyPlatform;
}

/** A shortcut chip: the keys in a small bordered `<kbd>`, tabular. */
export function ShortcutChip({ keys, platform }: ShortcutChipProps): ReactNode {
  return <kbd className="paper-shortcut-chip">{keysText(keys[platform], platform)}</kbd>;
}
