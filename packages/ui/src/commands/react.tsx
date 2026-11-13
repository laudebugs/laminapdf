// React's view of the command registry: a provider that holds the registry, the context commands
// run in and the platform the keys are written for, and listens for shortcuts; `useCommand` for a
// toolbar button or menu item; and `useShortcutMap` for the shortcut help and the desktop app's
// native menus. Every view reads the one registry, so none can drift from another.

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useSyncExternalStore,
} from "react";
import type { KeyPlatform, ShortcutKeys } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import type { CommandRegistry } from "./registry.js";
import { isBound, shortcutDispatcher } from "./shortcuts.js";
import type { AnyCommand, Availability, CommandContext, CommandGroup, CommandId } from "./types.js";

interface CommandsValue {
  readonly registry: CommandRegistry;
  readonly context: CommandContext;
  readonly keys: KeyPlatform;
}

const Commands = createContext<CommandsValue | undefined>(undefined);

/** Props of `CommandsProvider`. */
export interface CommandsProviderProps {
  readonly registry: CommandRegistry;
  /** What the commands read and drive: the platform, reader, viewer and the shell's handlers. */
  readonly context: CommandContext;
  /** How shortcut chips are written, and which keys `Mod` means: the platform the app runs on. */
  readonly keys: KeyPlatform;
  /** Whether single-key shortcuts are on (Settings can turn them off; default on). */
  readonly singleKeys?: boolean;
  /** Whether to listen for shortcuts on the window (default true; one provider per page should). */
  readonly listen?: boolean;
  readonly children: ReactNode;
}

/** Makes the registry and context available to `useCommand`, and dispatches shortcuts. */
export function CommandsProvider({
  registry,
  context,
  keys,
  singleKeys = true,
  listen = true,
  children,
}: CommandsProviderProps): ReactNode {
  // The dispatcher reads the latest context and setting, updated after each render.
  const latest = useRef(context);
  const single = useRef(singleKeys);
  useEffect(() => {
    latest.current = context;
    single.current = singleKeys;
  });
  useEffect(() => {
    if (!listen) return;
    const dispatch = shortcutDispatcher({
      registry,
      context: () => latest.current,
      platform: keys,
      singleKeys: () => single.current,
    });
    window.addEventListener("keydown", dispatch);
    return () => window.removeEventListener("keydown", dispatch);
  }, [registry, keys, listen]);
  const value = useMemo(() => ({ registry, context, keys }), [registry, context, keys]);
  return <Commands.Provider value={value}>{children}</Commands.Provider>;
}

/** The provider's registry, context and key platform. */
export function useCommands(): CommandsValue {
  const value = useContext(Commands);
  if (value === undefined) throw new Error("useCommands needs a CommandsProvider above it");
  return value;
}

/** Calls `listener` when the registry, the reader or the viewer changes. */
function subscribeTo(value: CommandsValue, listener: () => void): () => void {
  const stops = [
    value.registry.subscribe(listener),
    value.context.reader?.subscribe(listener),
    value.context.viewer?.subscribe(listener),
    value.context.edits?.subscribe(listener),
  ];
  return () => {
    for (const stop of stops) stop?.();
  };
}

/** The keys `command` shows on chips in `ctx`: its first binding bound there. */
export function chipKeys(command: AnyCommand, ctx: CommandContext): ShortcutKeys | undefined {
  const binding = command.shortcuts?.find((b) => isBound(b, ctx));
  return binding === undefined ? undefined : { mac: binding.mac, other: binding.other };
}

/** A command as a button or menu item shows it, and how to run it. */
export interface CommandView {
  readonly id: CommandId;
  readonly title: string;
  readonly shortcut?: ShortcutKeys;
  readonly availability: Availability;
  /** Runs the command with `args`; does nothing when it can't run now. */
  run(args?: unknown): void;
}

/** What a view shows of a command, as JSON: a string that stays the same while it does. */
function snapshotOf(registry: CommandRegistry, context: CommandContext, id: CommandId): string {
  const command = registry.get(id);
  if (command === undefined) return JSON.stringify({ title: id, availability: { state: "hidden" } });
  const shortcut = chipKeys(command, context);
  return JSON.stringify({
    title: registry.title(command, context),
    availability: registry.availability(id, context),
    ...(shortcut === undefined ? {} : { shortcut }),
  });
}

/**
 * The command `id` for a button or menu item: its title, shortcut, whether it can run now (kept
 * up to date as the reader and viewer change) and `run`. Re-renders only when those change.
 */
export function useCommand(id: CommandId): CommandView {
  const value = useCommands();
  const { registry, context } = value;
  const subscribe = useCallback((listener: () => void) => subscribeTo(value, listener), [value]);
  const read = () => snapshotOf(registry, context, id);
  // The view comes from the snapshot itself, so a memoised build (the React Compiler) can't keep
  // an availability the reader or viewer has since changed.
  const snapshot = useSyncExternalStore(subscribe, read, read);
  const shown = JSON.parse(snapshot) as Omit<CommandView, "id" | "run">;
  return {
    id,
    ...shown,
    run: (args?: unknown) => {
      registry.run(id, context, args === undefined ? {} : { args }).catch(() => undefined);
    },
  };
}

/** One group of the shortcut map: its title and its commands' keys. */
export interface ShortcutGroup {
  readonly group: CommandGroup;
  readonly title: string;
  readonly commands: readonly {
    readonly id: CommandId;
    readonly title: string;
    /** Each binding's keys; `on` names the one platform a binding exists on, when it isn't both. */
    readonly keys: readonly (ShortcutKeys & { readonly on?: KeyPlatform })[];
  }[];
}

/**
 * Every shortcut bound in `ctx`, by group in registry order: what the shortcut help lists (only
 * this platform's keys) and what the desktop app's menus take their accelerators from. Commands
 * the shell leaves out are left out here too.
 */
export function shortcutMap(registry: CommandRegistry, context: CommandContext): readonly ShortcutGroup[] {
  const groups = new Map<CommandGroup, ShortcutGroup["commands"][number][]>();
  for (const command of registry.list()) {
    if (registry.availability(command.id, context).state === "hidden") continue;
    const keys = (command.shortcuts ?? [])
      .filter((b) => isBound(b, context))
      .map((b) => ({ mac: b.mac, other: b.other, ...(b.on === undefined ? {} : { on: b.on }) }));
    if (keys.length === 0) continue;
    groups.set(command.group, [
      ...(groups.get(command.group) ?? []),
      { id: command.id, title: registry.title(command, context), keys },
    ]);
  }
  return [...groups].map(([group, commands]) => ({ group, title: message(`commandGroup.${group}`), commands }));
}

/** {@link shortcutMap} of the provider's registry and context, kept current. */
export function useShortcutMap(): readonly ShortcutGroup[] {
  const value = useCommands();
  const { registry, context } = value;
  const subscribe = useCallback((listener: () => void) => registry.subscribe(listener), [registry]);
  const version = useSyncExternalStore(
    subscribe,
    () => registry.version,
    () => registry.version,
  );
  // biome-ignore lint/correctness/useExhaustiveDependencies: the registry's version stands for its commands
  return useMemo(() => shortcutMap(registry, context), [version, registry, context]);
}
