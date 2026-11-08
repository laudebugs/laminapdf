// Shortcuts (ADR 0024 §3, design language §8.8): each declared once, on its command, matched
// against key events here, checked against each other and against the keys the browser, the OS
// and assistive tech keep, and dispatched from one keydown listener that leaves text fields alone.

import type { KeyPlatform } from "../components/shortcut-chip.js";
import type { CommandRegistry } from "./registry.js";
import type { AnyCommand, CommandContext, Shortcut } from "./types.js";

/** The two shells: the web app, where a browser keeps many keys, and the desktop app. */
export type Shell = "web" | "desktop";

/** The shell `ctx` runs in: the desktop app is the one that owns the keyboard. */
export const shellOf = (ctx: CommandContext): Shell => (ctx.platform.capabilities.ownsKeyboard ? "desktop" : "web");

/** A key event's parts, as `KeyboardEvent` has them. */
export interface KeyPress {
  readonly key: string;
  readonly code?: string;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly metaKey: boolean;
}

const MODIFIER_NAMES: Record<string, "ctrl" | "alt" | "shift" | "meta" | "mod"> = {
  ctrl: "ctrl",
  control: "ctrl",
  alt: "alt",
  option: "alt",
  shift: "shift",
  meta: "meta",
  cmd: "mod",
  mod: "mod",
};

/** One key's name, the same however it was written or typed: `=` is the `+` key unshifted. */
function keyName(key: string): string {
  const lower = key.length === 1 ? key.toLowerCase() : key.toLowerCase().replace(/^esc$/, "escape");
  return lower === "=" ? "+" : lower;
}

/** Whether `key` is one printable symbol, whose Shift is part of the character (`?`, `+`). */
const isSymbol = (key: string) => key.length === 1 && !/[a-z0-9]/i.test(key);

/** `ctrl+alt+shift+meta+key`, with the parts present, for comparing bindings and events. */
function canonical(mods: { ctrl: boolean; alt: boolean; shift: boolean; meta: boolean }, key: string): string {
  const name = keyName(key);
  const shift = mods.shift && !isSymbol(name);
  return [mods.ctrl && "ctrl", mods.alt && "alt", shift && "shift", mods.meta && "meta", name]
    .filter(Boolean)
    .join("+");
}

/** `"Mod+Shift+E"` on `platform` in canonical form, `Mod` read as ⌘ on macOS and Ctrl elsewhere. */
export function bindingKey(keys: string, platform: KeyPlatform): string {
  const parts = keys.split(/\+(?=.)/);
  const key = parts.pop() ?? "";
  const mods = { ctrl: false, alt: false, shift: false, meta: false };
  for (const part of parts) {
    const name = MODIFIER_NAMES[part.toLowerCase()];
    if (name === undefined) throw new Error(`unknown modifier ${part} in ${keys}`);
    if (name === "mod") mods[platform === "mac" ? "meta" : "ctrl"] = true;
    else mods[name] = true;
  }
  return canonical(mods, key);
}

/** A key event in canonical form. On macOS ⌥ changes the character, so a letter comes from its code. */
export function pressKey(event: KeyPress): string {
  const letter = event.altKey && /^Key[A-Z]$/.test(event.code ?? "") ? (event.code ?? "").slice(3) : event.key;
  return canonical({ ctrl: event.ctrlKey, alt: event.altKey, shift: event.shiftKey, meta: event.metaKey }, letter);
}

/** Whether a canonical binding holds Ctrl, Alt or ⌘: one a text field never types. */
const hasCommandModifier = (key: string) =>
  key
    .split("+")
    .slice(0, -1)
    .some((part) => part === "ctrl" || part === "alt" || part === "meta");

/** A key list entry: the keys on one or both platforms, in one or both shells, and why. */
interface KeyRule {
  readonly keys: string;
  readonly on?: KeyPlatform;
  readonly shells?: readonly Shell[];
  readonly why: string;
}

const WEB: readonly Shell[] = ["web"];

/**
 * Keys a command may never take. On the web: what the browser won't give up or users rely on
 * (§8.8's list, Firefox's Quick Find, closing and quitting). In the desktop app: only the OS's.
 */
export const NEVER_BIND: readonly KeyRule[] = [
  ...["Mod+W", "Mod+T", "Mod+N", "Mod+Q", "Mod+L", "Mod+D", "Mod+J", "Mod+Tab"].map((keys) => ({
    keys,
    shells: WEB,
    why: "the browser's tab, window, address-bar and download keys",
  })),
  { keys: "Mod+G", why: "find next in the browser and the OS" },
  { keys: "Mod+Shift+G", why: "find previous in the browser and the OS" },
  ...["F5", "F6", "F11", "F12"].map((keys) => ({
    keys,
    shells: WEB,
    why: "reload, focus regions, full screen, devtools",
  })),
  { keys: "/", shells: WEB, why: "Firefox's Quick Find" },
  { keys: "'", shells: WEB, why: "Firefox's Quick Find for links" },
  ...["Ctrl+Shift+I", "Ctrl+Shift+J", "Ctrl+Shift+C", "Ctrl+Shift+E"].map((keys) => ({
    keys,
    on: "other" as const,
    shells: WEB,
    why: "the browser's devtools and Firefox's network monitor",
  })),
  ...["Alt+Mod+I", "Alt+Mod+J", "Alt+Mod+C"].map((keys) => ({
    keys,
    on: "mac" as const,
    shells: WEB,
    why: "the browser's devtools",
  })),
  { keys: "Mod+H", on: "mac", why: "macOS hides the app" },
  { keys: "Mod+M", on: "mac", why: "macOS minimises the window" },
  { keys: "Mod+Space", on: "mac", why: "Spotlight" },
  { keys: "Mod+Tab", on: "mac", why: "macOS switches apps" },
  { keys: "Mod+.", on: "mac", shells: WEB, why: "stops loading in Safari" },
  { keys: "Mod+H", on: "other", shells: WEB, why: "the browser's history" },
  { keys: "Alt+F4", on: "other", why: "Windows closes the window" },
  { keys: "Alt+Tab", on: "other", why: "Windows switches apps" },
];

/**
 * Keys the browser or the OS gives a meaning that a command may take only on purpose: each must
 * be in that shell's `OVERRIDES` with a reason. (In the desktop app they are the platform's usual
 * app keys, which a webview would otherwise handle.)
 */
export const BROWSER_KEYS: readonly string[] = [
  "Mod+K",
  "Mod+F",
  "Mod+P",
  "Mod+S",
  "Mod+O",
  "Mod+C",
  "Mod+V",
  "Mod+X",
  "Mod+A",
  "Mod+Z",
  "Mod+R",
  "Mod+E",
  "Mod+U",
  "Mod+W",
  "Mod+Q",
  "Mod+[",
  "Mod+]",
  "Mod++",
  "Mod+-",
  "Mod+0",
  "Alt+ArrowLeft",
  "Alt+ArrowRight",
  "F1",
  "F3",
  "Shift+F3",
  "Shift+F10",
];

/** The browser and OS keys each shell takes on purpose, and why. */
export const OVERRIDES: Readonly<Record<Shell, readonly KeyRule[]>> = {
  web: [
    { keys: "Mod+K", why: "the command palette; Chrome and Firefox would focus their search box" },
    { keys: "Mod+F", why: "the reader's own find, which searches the whole document, not the drawn pages" },
    { keys: "Mod+P", why: "the reader's own print, which prints the document rather than the page around it" },
    { keys: "Mod+S", why: "saves an edit session" },
    { keys: "Mod+O", why: "opens a PDF in the reader rather than in a new tab" },
    { keys: "Mod+C", why: "copies the page's text from the text layer, in reading order" },
    { keys: "F3", why: "next match in the reader's find field" },
    { keys: "Shift+F3", why: "previous match in the reader's find field" },
    { keys: "Shift+F10", why: "the selection's own actions in place of the browser's context menu" },
  ],
  desktop: [
    { keys: "Mod+K", why: "the command palette" },
    { keys: "Mod+F", why: "find in the document" },
    { keys: "Mod+P", why: "print" },
    { keys: "Mod+S", why: "save" },
    { keys: "Mod+O", why: "open a file" },
    { keys: "Mod+C", why: "copy the selection" },
    { keys: "Mod+W", why: "closes the tab, as in other tabbed apps" },
    { keys: "Mod+[", why: "back, as in other apps with history" },
    { keys: "Mod+]", why: "forward" },
    { keys: "Alt+ArrowLeft", why: "back on Windows and Linux" },
    { keys: "Alt+ArrowRight", why: "forward on Windows and Linux" },
    { keys: "Mod++", why: "zoom in; there is no browser zoom around the app" },
    { keys: "Mod+-", why: "zoom out" },
    { keys: "Mod+0", why: "actual size" },
    { keys: "F1", why: "the keyboard shortcuts, as Help is elsewhere" },
    { keys: "F3", why: "next match in the find field" },
    { keys: "Shift+F3", why: "previous match in the find field" },
    { keys: "Shift+F10", why: "the selection's actions, the platform's context-menu key" },
  ],
};

/** The canonical keys of `rules` that apply to `shell` on `platform`. */
function keysOf(rules: readonly KeyRule[], shell: Shell, platform: KeyPlatform): Set<string> {
  return new Set(
    rules
      .filter((r) => (r.on === undefined || r.on === platform) && (r.shells === undefined || r.shells.includes(shell)))
      .map((r) => bindingKey(r.keys, platform)),
  );
}

/** Whether `binding` is bound in `shell`: its capability, when it needs one, is the shell's. */
function boundIn(binding: Shortcut, shell: Shell): boolean {
  return binding.requires === undefined || (binding.requires === "ownsKeyboard" && shell === "desktop");
}

/** The keys a check reads on one platform in one shell. */
interface KeyLists {
  readonly never: Set<string>;
  readonly browser: Set<string>;
  readonly allowed: Set<string>;
}

/** What is wrong with one binding's `keys` on its own: a never-bind key, an unexplained override, AltGr. */
function keyProblems(keys: string, where: string, lists: KeyLists, platform: KeyPlatform): string[] {
  const problems: string[] = [];
  if (lists.never.has(keys)) problems.push(`${where} takes a key that may never be bound`);
  if (lists.browser.has(keys) && !lists.allowed.has(keys)) {
    problems.push(`${where} overrides a browser or OS key without a reason`);
  }
  if (platform === "other" && /^ctrl\+alt\+(?:shift\+)?[a-z]$/.test(keys)) {
    problems.push(`${where} is Ctrl+Alt+letter, which types characters on many keyboards`);
  }
  return problems;
}

/** A binding already seen: which command, in which scope. */
type Taken = { readonly id: string; readonly scope: Shortcut["scope"] };

/** The commands among `others` that `binding` of command `id` collides with: the same keys where scopes meet. */
function collisions(others: readonly Taken[], id: string, binding: Shortcut): string[] {
  return others
    .filter((o) => o.id !== id && (o.scope === binding.scope || o.scope === "global" || binding.scope === "global"))
    .map((o) => o.id);
}

/** `shortcutProblems` on one platform. */
function platformProblems(commands: readonly AnyCommand[], shell: Shell, platform: KeyPlatform): string[] {
  const lists: KeyLists = {
    never: keysOf(NEVER_BIND, shell, platform),
    browser: new Set(BROWSER_KEYS.map((k) => bindingKey(k, platform))),
    allowed: keysOf(OVERRIDES[shell], shell, platform),
  };
  const taken = new Map<string, Taken[]>();
  const problems: string[] = [];
  for (const command of commands) {
    for (const binding of (command.shortcuts ?? []).filter((b) => boundIn(b, shell))) {
      const keys = bindingKey(binding[platform], platform);
      const where = `${command.id} (${binding[platform]}, ${platform}, ${shell})`;
      const others = taken.get(keys) ?? [];
      problems.push(
        ...keyProblems(keys, where, lists, platform),
        ...collisions(others, command.id, binding).map((other) => `${where} shares its keys with ${other}`),
      );
      taken.set(keys, [...others, { id: command.id, scope: binding.scope }]);
    }
  }
  return problems;
}

/**
 * What is wrong with the bindings of `commands` in `shell`, on both platforms: a binding to a key
 * that may never be bound, a browser or OS key taken without an override, Ctrl+Alt+letter (AltGr
 * on many layouts), or two commands sharing keys where their scopes meet. Empty when all is well.
 */
export function shortcutProblems(commands: readonly AnyCommand[], shell: Shell): string[] {
  return [...platformProblems(commands, shell, "mac"), ...platformProblems(commands, shell, "other")];
}

/** Whether `target` takes typing: a text field, a text area, a select or editable content. */
function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target instanceof HTMLInputElement) {
    return !["button", "checkbox", "radio", "range", "submit", "reset", "color", "file"].includes(target.type);
  }
  return (
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    target.closest("[contenteditable]:not([contenteditable=false])") !== null
  );
}

/** Options of `shortcutDispatcher`. */
export interface DispatcherOptions {
  readonly registry: CommandRegistry;
  /** The context to evaluate and run commands in, read at each key press. */
  readonly context: () => CommandContext;
  /** How the keys are written: the platform the app runs on. */
  readonly platform: KeyPlatform;
  /** Whether single-key shortcuts are on (Settings can turn them off, WCAG 2.1.4). */
  readonly singleKeys?: () => boolean;
}

/**
 * A keydown handler that runs the command bound to the keys pressed, if it can run now, and
 * prevents the browser's own handling. It leaves alone events already handled, typing in a field
 * (unless the keys hold Ctrl, Alt or ⌘, or the field is the binding's own scope) and keys outside
 * a binding's scope.
 */
export function shortcutDispatcher(options: DispatcherOptions): (event: KeyboardEvent) => void {
  return (event) => {
    if (event.defaultPrevented || event.isComposing) return;
    const ctx = options.context();
    const found = boundCommand(options, ctx, event);
    if (found === undefined || !allowed(options, event, found.binding)) return;
    if (options.registry.availability(found.command.id, ctx).state !== "available") return;
    event.preventDefault();
    // A shortcut has no arguments to get wrong, and a refusal has nowhere to show.
    options.registry.run(found.command.id, ctx).catch(() => undefined);
  };
}

/** The first command with a binding to the keys of `event` in the scope it was pressed in. */
function boundCommand(
  options: DispatcherOptions,
  ctx: CommandContext,
  event: KeyboardEvent,
): { command: AnyCommand; binding: Shortcut } | undefined {
  const shell = shellOf(ctx);
  const pressed = pressKey(event);
  const region = event.target instanceof Element ? event.target.closest("[data-shortcut-scope]") : null;
  const scope = region?.getAttribute("data-shortcut-scope");
  const matches = (b: Shortcut) =>
    b.dispatch !== false &&
    boundIn(b, shell) &&
    bindingKey(b[options.platform], options.platform) === pressed &&
    (b.scope === "global" || b.scope === scope);
  for (const command of options.registry.list()) {
    const binding = command.shortcuts?.find(matches);
    if (binding !== undefined) return { command, binding };
  }
  return undefined;
}

/**
 * Whether `binding` may act on `event`: keys without Ctrl, Alt or ⌘ never act while typing in a
 * field (but the find field's own Enter and F3 do), nor when single-key shortcuts are off.
 */
function allowed(options: DispatcherOptions, event: KeyboardEvent, binding: Shortcut): boolean {
  if (hasCommandModifier(pressKey(event)) || binding.scope === "find-field") return true;
  return !isEditable(event.target) && options.singleKeys?.() !== false;
}
