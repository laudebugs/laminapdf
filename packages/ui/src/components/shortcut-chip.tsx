// The shortcut chip (§7.10): a command's keys as the platform writes them. macOS runs glyphs
// together in its modifier order (⌥⌘E); Windows and Linux spell the modifiers out with spaces
// (Ctrl Shift E). The app says which platform it runs on, so this component never checks.

import { Fragment, type ReactNode } from "react";
import { Keyboard } from "react-aria-components";

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
  alt: { names: ["alt", "option", "opt"], mac: "⌥", other: "Alt" },
  shift: { names: ["shift"], mac: "⇧", other: "Shift" },
  meta: { names: ["meta", "win", "super"], mac: "⌘", other: "Win" },
  mod: { names: ["mod", "cmd", "command"], mac: "⌘", other: "Ctrl" },
} as const;

type Modifier = keyof typeof MODIFIERS;

const BY_NAME = new Map<string, Modifier>(
  (Object.keys(MODIFIERS) as Modifier[]).flatMap((m) => MODIFIERS[m].names.map((n) => [n, m] as const)),
);

/** The order each platform writes modifiers in: macOS puts ⌘ last, Windows puts Ctrl first. */
const ORDER: Record<KeyPlatform, readonly Modifier[]> = {
  mac: ["ctrl", "alt", "shift", "meta", "mod"],
  other: ["mod", "ctrl", "meta", "alt", "shift"],
};

/** Keys whose names read better as a glyph or a short word. */
const KEYS: Record<string, { mac: string; other: string }> = {
  escape: { mac: "esc", other: "Esc" },
  esc: { mac: "esc", other: "Esc" },
  enter: { mac: "↩", other: "Enter" },
  backspace: { mac: "⌫", other: "Backspace" },
  delete: { mac: "⌦", other: "Delete" },
  tab: { mac: "⇥", other: "Tab" },
  space: { mac: "Space", other: "Space" },
  " ": { mac: "Space", other: "Space" },
  pageup: { mac: "PgUp", other: "PgUp" },
  pagedown: { mac: "PgDn", other: "PgDn" },
  home: { mac: "Home", other: "Home" },
  end: { mac: "End", other: "End" },
  arrowup: { mac: "↑", other: "↑" },
  arrowdown: { mac: "↓", other: "↓" },
  arrowleft: { mac: "←", other: "←" },
  arrowright: { mac: "→", other: "→" },
};

/**
 * Glyphs that Lamina Sans draws below the middle of its capitals (⌘, and the arrows, which sit on
 * the maths axis). The chip lifts them by `--paper-shortcut-chip-glyph-lift` so each key lines up
 * with the letters beside it.
 */
const LOW_GLYPHS = new Set(["⌘", "↑", "↓", "←", "→"]);

/**
 * `"Mod+Shift+E"` written for `platform`, one entry per key: `⇧`, `⌘`, `E`, or `Ctrl`, `Shift`, `E`.
 * A modifier this table doesn't know is kept, as written, before the key, so the chip never shows
 * fewer keys than the binding.
 */
function keyNames(keys: string, platform: KeyPlatform): string[] {
  // Split on each "+" that has something after it, so "Mod++" is Mod and "+".
  const parts = keys.split(/\+(?=.)/);
  const key = parts.pop() ?? "";
  const held = new Set<Modifier>();
  const unknown: string[] = [];
  for (const part of parts) {
    const modifier = BY_NAME.get(part.toLowerCase());
    if (modifier === undefined) unknown.push(part);
    else held.add(modifier);
  }
  const modifiers = ORDER[platform].filter((m) => held.has(m)).map((m) => MODIFIERS[m][platform]);
  const named = KEYS[key.toLowerCase()];
  const last = named?.[platform] ?? key.charAt(0).toUpperCase() + key.slice(1);
  // Windows writes Ctrl once, whether it came from Ctrl or Mod.
  return [...new Set(modifiers), ...unknown, last];
}

/** Props of `ShortcutChip`. */
export interface ShortcutChipProps {
  /** The command's keys. */
  keys: ShortcutKeys;
  /** The platform the app runs on, from its `Platform` adapter. */
  platform: KeyPlatform;
}

/**
 * A shortcut chip: the keys in a small bordered `<kbd>`, tabular. It is React Aria's `Keyboard`,
 * so inside a menu item it describes the item (`aria-describedby`). Each key is a span: macOS
 * writes its keys with no space between them (`⌘V`), and the CSS keeps them apart; elsewhere a
 * space separates them, so the chip's text reads the same as the platform writes it.
 */
export function ShortcutChip({ keys, platform }: ShortcutChipProps): ReactNode {
  const names = keyNames(keys[platform], platform);
  return (
    <Keyboard className="paper-shortcut-chip" data-platform={platform}>
      {names.map((name, i) => (
        // Keys repeat only when a binding names one twice, which is harmless.
        // biome-ignore lint/suspicious/noArrayIndexKey: the keys never reorder.
        <Fragment key={i}>
          {platform === "other" && i > 0 ? " " : null}
          <span className="paper-shortcut-chip-key" data-low={LOW_GLYPHS.has(name) || undefined}>
            {name}
          </span>
        </Fragment>
      ))}
    </Keyboard>
  );
}
