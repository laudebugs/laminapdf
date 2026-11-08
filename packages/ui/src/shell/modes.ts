// The shell's modes (ADR 0024 §2): each registers its id, title, icon and the toolbar shown under the
// top bar while it is the current mode. The registry holds the current mode; switching calls the old
// mode's `leave`, so a selection that only makes sense in that mode (a selected comment) is cleared,
// then the new mode's `enter`. Reader v0.1 registers Read alone, so the switcher stays hidden until a
// second mode exists.

import type { IconName } from "@laminapdf/icons";
import type { ReactNode } from "react";
import type { ExternalStore } from "../state/hooks.js";

/** One mode, as it registers. */
export interface ShellMode {
  /** `read`, `comment`, `edit`. */
  readonly id: string;
  /** Its name in the switcher ("Read mode"). */
  readonly title: string;
  readonly icon?: IconName;
  /** The secondary toolbar while this mode is current; none for a mode without tools (Read). */
  readonly toolbar?: () => ReactNode;
  /** Runs when the user switches to this mode. */
  readonly enter?: () => void;
  /** Clears what only this mode can select, when the user switches away. */
  readonly leave?: () => void;
}

/** The registry's state. */
export interface ModesState {
  /** The modes, in the order they registered. */
  readonly modes: readonly ShellMode[];
  /** The current mode's id, or undefined before one registers. */
  readonly current: string | undefined;
}

/** The modes and the current one. */
export class ModeRegistry implements ExternalStore<ModesState> {
  #state: ModesState = { modes: [], current: undefined };
  readonly #listeners = new Set<() => void>();

  getState(): ModesState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Adds `mode`, current if it is the first; returns the function that removes it. */
  register(mode: ShellMode): () => void {
    if (this.#state.modes.some((m) => m.id === mode.id)) throw new Error(`mode ${mode.id} is already registered`);
    const modes = [...this.#state.modes, mode];
    this.#set({ modes, current: this.#state.current ?? mode.id });
    return () => {
      const rest = this.#state.modes.filter((m) => m !== mode);
      const current = this.#state.current === mode.id ? rest[0]?.id : this.#state.current;
      this.#set({ modes: rest, current });
    };
  }

  /** The current mode, or undefined. */
  current(): ShellMode | undefined {
    return this.#state.modes.find((m) => m.id === this.#state.current);
  }

  /** Makes `id` the current mode: the old mode's `leave`, then the new one's `enter`. Unknown ids are ignored. */
  switchTo(id: string): void {
    const next = this.#state.modes.find((m) => m.id === id);
    if (next === undefined || id === this.#state.current) return;
    this.current()?.leave?.();
    this.#set({ current: id });
    next.enter?.();
  }

  #set(change: Partial<ModesState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}
