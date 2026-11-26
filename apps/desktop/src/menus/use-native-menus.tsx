// The window's side of the native menus: it tells the main process which of the menus'
// commands it can run now, so the menus show the focused window's state, and runs the commands
// chosen from the menus through the registry, as a key press or the palette would. A key press
// that reaches a command through both the web view and a menu's accelerator runs it once (the
// registry's `source`).

import { type CommandContext, type CommandId, type CommandRegistry, isEditable } from "@laminapdf/ui";
import { useEffect, useLayoutEffect, useRef } from "react";
import MENUS from "../../src-tauri/menus.json" with { type: "json" };
import type { DesktopShell } from "../engine/desktop-engine-client.ts";
import { stopper } from "../engine/stopper.ts";
import { type MenusJson, menuCommandIds } from "./menus.ts";

/** A menu command for this window: `src-tauri/src/menu.rs`'s `COMMAND_EVENT`. */
const MENU_COMMAND_EVENT = "menu://command";

/** What `MENU_COMMAND_EVENT` carries. */
interface MenuCommand {
  readonly id: string;
  /** A choice's value, a recent file's id, or a window's label. */
  readonly arg?: string;
}

/** One command's state, as `menu_state` takes it: `checked` for a toggle or a choice that has one. */
export interface CommandState {
  readonly id: string;
  readonly enabled: boolean;
  readonly checked?: boolean | string;
}

/** The menu commands a text field with the focus does itself. */
const FIELD_EDITS: Partial<Record<CommandId, "undo" | "redo">> = { "edit.undo": "undo", "edit.redo": "redo" };

/** The ids of the commands the menus list. */
const IDS = menuCommandIds(MENUS as MenusJson) as readonly CommandId[];

/** Which of the menus' commands can run in `context` now, and which toggles and choices are on. */
export function menuState(registry: CommandRegistry, context: CommandContext): CommandState[] {
  return IDS.map((id) => {
    const checked = registry.get(id)?.checked?.(context);
    return {
      id,
      enabled: registry.availability(id, context).state === "available",
      ...(checked === undefined ? {} : { checked }),
    };
  });
}

/**
 * Runs `command`, chosen from a menu: through the registry, or, for an action that isn't a
 * command (Move tab to window ▸), through its handler. A command that can't run now does nothing.
 */
export function runMenuCommand(registry: CommandRegistry, context: CommandContext, command: MenuCommand): void {
  const id = command.id as CommandId;
  // In a text field, Edit ▸ Undo and Redo (and their keys, which the menu takes first) are the
  // field's, as the OS's own Edit menu would make them.
  const inField = FIELD_EDITS[id];
  const focused = inField === undefined ? undefined : globalThis.document?.activeElement;
  if (inField !== undefined && focused != null && isEditable(focused)) {
    document.execCommand(inField);
    return;
  }
  if (registry.get(id) === undefined) {
    void context.actions?.[id]?.(command.arg);
    return;
  }
  const input = command.arg === undefined ? {} : { args: command.arg };
  registry.run(id, context, { ...input, source: "menu" }).catch(() => undefined);
}

/**
 * Keeps the main process told of this window's commands' state, and runs the commands its menus
 * send here. Without a shell (a browser), nothing.
 */
export function useNativeMenus(
  shell: DesktopShell | undefined,
  registry: CommandRegistry,
  context: CommandContext,
): void {
  const latest = useRef({ registry, context });
  useLayoutEffect(() => {
    latest.current = { registry, context };
  }, [registry, context]);

  useEffect(() => {
    if (shell === undefined) return;
    let last = "";
    let frame: number | undefined;
    // Many changes can come at once (a page turn, a zoom step): one report a frame, and only
    // when something the menus show changed.
    const report = () => {
      if (frame !== undefined) return;
      frame = requestAnimationFrame(() => {
        frame = undefined;
        const state = menuState(registry, context);
        const text = JSON.stringify(state);
        if (text === last) return;
        last = text;
        shell.invoke("menu_state", { state }).catch(() => undefined);
      });
    };
    report();
    const stops = [
      registry.subscribe(report),
      context.reader?.subscribe(report),
      context.viewer?.subscribe(report),
      context.viewer?.history?.subscribe(report),
      context.edits?.subscribe(report),
      context.layout?.subscribe(report),
    ];
    return () => {
      if (frame !== undefined) cancelAnimationFrame(frame);
      for (const stop of stops) stop?.();
    };
  }, [shell, registry, context]);

  useEffect(() => {
    if (shell === undefined) return;
    return stopper(
      shell.listen<MenuCommand>(MENU_COMMAND_EVENT, (command) =>
        runMenuCommand(latest.current.registry, latest.current.context, command),
      ),
      "listening for menu commands",
    );
  }, [shell]);
}
