// The command registry's types (ADR 0024 §3). A command is one action the shell can take, with
// one id across every surface: the palette, toolbars, menus, context menus, native menus and the
// shortcut help all read it from the registry, so labels, shortcuts and enabled state can't drift.

import type { IconName } from "@laminapdf/icons";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import type { EditControls } from "../edit/edit-controls.js";
import type { Capability, Platform } from "../platform/types.js";
import type { ViewerControls } from "../reader/viewer-controls.js";
import type { ExternalStore } from "../state/hooks.js";
import type { ReaderState } from "../state/reader-machine.js";

/** `namespace.verb-object`, kebab-case, one dot: `file.save-copy`, `view.zoom-in`. */
export type CommandId = `${string}.${string}`;

/** The menu and palette section a command sits in. */
export type CommandGroup = "file" | "navigate" | "view" | "find" | "edit" | "privacy" | "help" | "app";

/**
 * Where a binding's keys act. `global`: anywhere, inputs included when the keys hold a modifier.
 * Any other scope: only inside an element marked `data-shortcut-scope` with that name, and never
 * as a single key in a text field. The canvas is `page`; the find field is `find-field`.
 */
export type ShortcutScope = "global" | "page" | "find-field" | "selection";

/** One key binding of a command. */
export interface Shortcut {
  /** The keys on macOS: `"Mod+K"`, `"?"`, `"F3"`, where `Mod` is ⌘. */
  readonly mac: string;
  /** The keys on Windows and Linux, where `Mod` is Ctrl. */
  readonly other: string;
  readonly scope: ShortcutScope;
  /** A capability the binding needs; `ownsKeyboard` for keys a browser keeps (⌘W, F1). */
  readonly requires?: Capability;
  /**
   * False when something else handles the keys (the layer stack's Escape): the binding is still
   * shown on chips and in the help, and still checked for collisions, but never dispatched.
   */
  readonly dispatch?: false;
  /**
   * False when a text field keeps the keys for itself even though they hold a modifier: Undo's
   * ⌘Z undoes the typing in a field, not the document's last change.
   */
  readonly fields?: false;
  /** The one platform the binding exists on, when it isn't both: Redo's extra Ctrl+Shift+Z. */
  readonly on?: KeyPlatform;
}

/** Where a command has a visible home besides the palette (ADR 0024: every command has one). */
export type CommandHome =
  | "top-bar"
  | "zoom-control"
  | "page-indicator"
  | "file-menu"
  | "edit-menu"
  | "view-menu"
  | "help-menu"
  | "view-settings"
  | "home"
  | "settings"
  | "password-prompt"
  | "status-bar"
  | "find-bar"
  | "context-menu"
  | "mini-toolbar"
  | "mode-switcher"
  | "history-panel"
  | "threads-panel"
  | "layer";

/** What a parser made of typed arguments. */
export type ParseResult<T> =
  | { readonly ok: true; readonly value: T; readonly summary: string }
  | { readonly ok: false; readonly error: string };

/** A command's typed arguments, which the palette parses inline (`zoom 150`, `go to 12`). */
export interface ArgsSchema<Args> {
  /** What the palette asks for when the command is chosen without them: "A page number or label". */
  readonly prompt: string;
  /** Words besides the title that start an inline argument: `go to` in `go to 12`. */
  readonly prefixes?: readonly string[];
  /** True when the command also runs without arguments (Print prints every page). */
  readonly optional?: boolean;
  /** True for a secret (a password): never typed into the palette, whose text stays on screen. */
  readonly secret?: boolean;
  /** Reads the typed text; the error says what to type instead. */
  parse(text: string, ctx: CommandContext): ParseResult<Args>;
}

/** What a command can read and drive when it is evaluated or run. */
export interface CommandContext {
  readonly platform: Platform;
  /** The reader's lifecycle, with the actions the reader's own commands call. */
  readonly reader?: ExternalStore<ReaderState> & {
    /** Asks the platform for a file and opens it. */
    openFromPicker(): Promise<void>;
    /** Retries opening the locked document with `password`. */
    submitPassword(password: string): Promise<void>;
  };
  /** The viewer showing the open document. */
  readonly viewer?: ViewerControls;
  /** The open document's edit session: what Undo, Redo and the History tab drive. */
  readonly edits?: EditControls;
  /** The document's page labels, one per page, when it has any. */
  readonly labels?: readonly string[];
  /**
   * The shell's handlers for commands whose work lives outside this package (find, print, the
   * sidebar), by command id. A delegated command the shell has no handler for is left out of
   * every view, as a capability the platform can never have is.
   */
  readonly actions?: Readonly<Partial<Record<CommandId, (args: unknown) => void | Promise<void>>>>;
}

/** One action, as the registry holds it. */
export interface Command<Args = undefined> {
  readonly id: CommandId;
  /** Its name in every view; a function where it differs by platform ("Download a copy" on the web). */
  readonly title: string | ((ctx: CommandContext) => string);
  /** Extra words the palette matches: `["turn"]` for Rotate view. */
  readonly keywords?: readonly string[];
  readonly group: CommandGroup;
  readonly shortcuts?: readonly Shortcut[];
  readonly icon?: IconName;
  /** A capability the command needs (ADR 0010): hidden, or disabled with the platform's reason. */
  readonly requires?: Capability;
  /** True when the shell's `actions` does the work: without a handler the command is left out. */
  readonly delegated?: true;
  /** Where it has a visible home besides the palette. */
  readonly home: readonly CommandHome[];
  readonly args?: ArgsSchema<Args>;
  /** `true`, or why it can't run now ("Open a document first"). */
  enabled?(ctx: CommandContext): true | string;
  /** Does the command's work, with its parsed arguments. */
  run(ctx: CommandContext, args: Args): void | Promise<void>;
}

/** Any command, whatever its arguments: what the registry stores. */
// biome-ignore lint/suspicious/noExplicitAny: a registry of commands with different argument types
export type AnyCommand = Command<any>;

/** Whether a command can run now: available, disabled with a reason, or left out of every view. */
export type Availability =
  | { readonly state: "available" }
  | { readonly state: "disabled"; readonly reason: string }
  | { readonly state: "hidden" };
