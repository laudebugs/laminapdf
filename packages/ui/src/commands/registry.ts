// The one command registry (ADR 0024 §3): what every view of the shell's actions reads. It holds
// the commands, says whether each can run now (its capability, the shell's handler, its own
// `enabled`), and runs one, refusing a command that can't run now or arguments that don't parse.

import { capabilityState } from "../platform/context.js";
import type { AnyCommand, Availability, CommandContext, CommandId } from "./types.js";

/** Where a run came from: a key press in the web view, or a native menu item or its keys. */
export type RunSource = "keys" | "menu";

/**
 * How long after a run from one source the same command's run from the other is its echo: one
 * key press reaching both the web view and a native menu's accelerator, never two choices.
 */
export const ECHO_MS = 300;

/** Why `run` refused: the command is unknown, can't run now, or its arguments don't parse. */
export class CommandError extends Error {
  readonly id: CommandId;
  constructor(id: CommandId, reason: string) {
    super(reason);
    this.name = "CommandError";
    this.id = id;
  }
}

/** The registry: register commands, look them up, and evaluate or run them in a context. */
export class CommandRegistry {
  readonly #commands = new Map<CommandId, AnyCommand>();
  readonly #listeners = new Set<() => void>();
  #version = 0;
  #lastRun:
    | { readonly id: CommandId; readonly source: RunSource; readonly at: number; readonly echoed: boolean }
    | undefined;

  /** Adds `command`; returns the function that removes it. An id can be registered once. */
  register(command: AnyCommand): () => void {
    if (this.#commands.has(command.id)) throw new Error(`the command ${command.id} is already registered`);
    if (!/^[a-z]+(?:-[a-z]+)*\.[a-z]+(?:-[a-z]+)*$/.test(command.id)) {
      throw new Error(`the command id ${command.id} isn't namespace.verb-object in kebab-case`);
    }
    this.#commands.set(command.id, command);
    this.#changed();
    return () => this.unregister(command.id);
  }

  /** Removes the command `id`, if registered. */
  unregister(id: CommandId): void {
    if (this.#commands.delete(id)) this.#changed();
  }

  /** The command `id`, or undefined. */
  get(id: CommandId): AnyCommand | undefined {
    return this.#commands.get(id);
  }

  /** Every command, in the order registered. */
  list(): readonly AnyCommand[] {
    return [...this.#commands.values()];
  }

  /** Changes each time a command is added or removed, for views that cache what they read. */
  get version(): number {
    return this.#version;
  }

  /** Calls `listener` when a command is added or removed; returns the function that stops it. */
  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Whether `id` can run in `ctx`: its capability first, then the shell's handler, then `enabled`. */
  availability(id: CommandId, ctx: CommandContext): Availability {
    const command = this.#commands.get(id);
    if (command === undefined) return { state: "hidden" };
    if (command.requires !== undefined) {
      const gate = capabilityState(ctx.platform, command.requires);
      if (gate.state !== "available") return gate;
    }
    if (command.delegated === true && ctx.actions?.[id] === undefined) return { state: "hidden" };
    const enabled = command.enabled?.(ctx) ?? true;
    return enabled === true ? { state: "available" } : { state: "disabled", reason: enabled };
  }

  /** The title of `command` in `ctx`. */
  title(command: AnyCommand, ctx: CommandContext): string {
    return typeof command.title === "function" ? command.title(ctx) : command.title;
  }

  /**
   * Runs `id` with `args`, or with `text` parsed by its schema. Rejects with a `CommandError`
   * when the command can't run now or its arguments are missing or don't parse; nothing runs then.
   *
   * `source` says where the run came from. One key press can reach a command twice in the
   * desktop app, through the web view's shortcut and the native menu's accelerator: a run from
   * one source within {@link ECHO_MS} of the same command's run from the other is that echo,
   * and does nothing.
   */
  async run(
    id: CommandId,
    ctx: CommandContext,
    input: { args?: unknown; text?: string; source?: RunSource } = {},
  ): Promise<void> {
    const command = this.#commands.get(id);
    if (command === undefined) throw new CommandError(id, `there is no command ${id}`);
    const available = this.availability(id, ctx);
    if (available.state === "hidden") throw new CommandError(id, `${id} isn't available here`);
    if (available.state === "disabled") throw new CommandError(id, available.reason);
    let args = input.args;
    if (input.text !== undefined && command.args !== undefined) {
      const parsed = command.args.parse(input.text, ctx);
      if (!parsed.ok) throw new CommandError(id, parsed.error);
      args = parsed.value;
    }
    if (args === undefined && command.args !== undefined && command.args.optional !== true) {
      throw new CommandError(id, command.args.prompt);
    }
    if (input.source !== undefined && this.#echo(id, input.source)) return;
    await command.run(ctx, args);
  }

  /** Whether a run of `id` from `source` now is the echo of one from the other source just before. */
  #echo(id: CommandId, source: RunSource): boolean {
    const now = Date.now();
    const last = this.#lastRun;
    const echo =
      last !== undefined && !last.echoed && last.id === id && last.source !== source && now - last.at < ECHO_MS;
    // A run has one echo at most: after it, the next run from either source is another press.
    this.#lastRun = echo ? { ...last, echoed: true } : { id, source, at: now, echoed: false };
    return echo;
  }

  #changed(): void {
    this.#version += 1;
    for (const listener of this.#listeners) listener();
  }
}
