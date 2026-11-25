// The shell's handlers for the commands it does (ADR 0024 §3): the rail and focus-mode toggles, and
// one `mode.<id>` per registered mode. The reader's command set declares them delegated, so they
// stay out of every view until a shell passes these in its `CommandContext.actions`.

import type { CommandContext, CommandId } from "../commands/types.js";
import type { ShellLayout } from "./layout.js";
import type { ModeRegistry, ShellMode } from "./modes.js";

/** The commands' handlers, by id, for `CommandContext.actions`. */
export type ShellActions = NonNullable<CommandContext["actions"]>;

/**
 * `layout`'s and `modes`' handlers. `rail` says whether the shell has rail tabs to show: without
 * them `view.toggle-sidebar` gets no handler, so the command stays out of every view. Pass `list`
 * from `useStoreState(modes, (s) => s.modes)` when memoised under React Compiler so mode changes
 * are tracked in the dependency graph.
 */
export function shellActions(
  layout: ShellLayout,
  modes: ModeRegistry,
  { rail, list }: { rail: boolean; list?: readonly ShellMode[] },
): ShellActions {
  const actions: Partial<Record<CommandId, (args: unknown) => void>> = {
    "view.focus-mode": () => layout.toggleFocus(),
  };
  if (rail) actions["view.toggle-sidebar"] = () => layout.toggleRail();
  for (const mode of list ?? modes.getState().modes) {
    actions[`mode.${mode.id}`] = () => modes.switchTo(mode.id);
  }
  return actions;
}
