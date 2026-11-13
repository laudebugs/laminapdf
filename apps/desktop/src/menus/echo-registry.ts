// The desktop app's command registry. One key press can reach a command twice here: through the
// web view's shortcut and through the native menu's accelerator for the same keys. A run from one
// source within ECHO_MS of the same command's run from the other is that echo, and does nothing.

import { type CommandContext, type CommandId, CommandRegistry, type RunSource } from "@laminapdf/ui";

/**
 * How long after a run from one source the same command's run from the other is its echo: one
 * key press reaching both the web view and a native menu's accelerator, never two choices.
 */
const ECHO_MS = 300;

/** The registry, dropping a key press's echo from the other source. */
export class EchoRegistry extends CommandRegistry {
  #lastRun: { readonly id: CommandId; readonly source: RunSource; readonly at: number; echoed: boolean } | undefined;

  override async run(
    id: CommandId,
    ctx: CommandContext,
    input: { args?: unknown; text?: string; source?: RunSource } = {},
  ): Promise<void> {
    // A run that can't happen leaves nothing to echo: the registry rejects it as before.
    if (
      input.source !== undefined &&
      this.availability(id, ctx).state === "available" &&
      this.#echo(id, input.source)
    ) {
      return;
    }
    await super.run(id, ctx, input);
  }

  /** Whether a run of `id` from `source` now is the echo of one from the other source just before. */
  #echo(id: CommandId, source: RunSource): boolean {
    const now = Date.now();
    const last = this.#lastRun;
    if (last !== undefined && !last.echoed && last.id === id && last.source !== source && now - last.at < ECHO_MS) {
      // A run has one echo at most: after it, the next run from either source is another press.
      last.echoed = true;
      return true;
    }
    this.#lastRun = { id, source, at: now, echoed: false };
    return false;
  }
}
