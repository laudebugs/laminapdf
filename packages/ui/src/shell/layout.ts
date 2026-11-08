// The shell's layout state (ADR 0024 §1, UI/UX plan §5.1): whether the left rail is open, its width
// and tab, whether focus mode is on, and what the inspector shows. An external store, so the shell,
// the commands and the status bar read one state. The rail's state is kept in the platform's
// storage between visits; focus mode lasts the window only, as the design says (R16).

import { COMPONENT } from "@laminapdf/tokens";
import type { PlatformStorage } from "../platform/types.js";
import type { ExternalStore } from "../state/hooks.js";

/** The rail's narrowest width, in CSS pixels: token `rail-width-min`. */
export const RAIL_MIN_WIDTH = COMPONENT["rail-width-min"];
/** The rail's widest width: twice the wide rail (`rail-width-wide`), so a page thumbnail still fits beside the canvas. */
export const RAIL_MAX_WIDTH = 2 * COMPONENT["rail-width-wide"];
/** The rail's width before the user resizes it: token `rail-width`. */
export const RAIL_DEFAULT_WIDTH = COMPONENT["rail-width"];
/** How far one arrow key press resizes the rail. */
export const RAIL_STEP = 16;

/** The storage key of the rail's kept state. */
const STORAGE_KEY = "shell.rail";
/** How long the rail's state rests before it is kept, so a drag writes once, not once a frame. */
const KEEP_AFTER_MS = 250;

/** What the inspector shows: a selection (which closes when it is deselected), or a panel opened on purpose. */
export interface InspectorContent {
  /** What it is, such as `repairs` or `selection`. */
  readonly id: string;
  /** True when a selection opened it: deselecting closes it, and it is the one panel focus mode shows. */
  readonly selection: boolean;
}

/** The shell's layout. */
export interface ShellLayoutState {
  readonly rail: {
    readonly open: boolean;
    readonly width: number;
    /** The selected tab, or undefined for the rail's first. */
    readonly tab: string | undefined;
  };
  /** Focus mode: the rail and the secondary toolbar hidden. */
  readonly focus: boolean;
  /** The inspector's content, or undefined when it is closed. */
  readonly inspector: InspectorContent | undefined;
}

/** `width` within the rail's limits, as a whole number of pixels. */
export function clampRailWidth(width: number): number {
  if (!Number.isFinite(width)) return RAIL_DEFAULT_WIDTH;
  return Math.round(Math.min(RAIL_MAX_WIDTH, Math.max(RAIL_MIN_WIDTH, width)));
}

/** The rail's kept state read back, or undefined when it isn't one this version wrote. */
function readKept(value: unknown): ShellLayoutState["rail"] | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const { open, width, tab } = value as Record<string, unknown>;
  if (typeof open !== "boolean" || typeof width !== "number") return undefined;
  return { open, width: clampRailWidth(width), tab: typeof tab === "string" ? tab : undefined };
}

/** The shell's layout store. */
export class ShellLayout implements ExternalStore<ShellLayoutState> {
  #state: ShellLayoutState = {
    rail: { open: false, width: RAIL_DEFAULT_WIDTH, tab: undefined },
    focus: false,
    inspector: undefined,
  };
  readonly #listeners = new Set<() => void>();
  readonly #deselected = new Set<() => void>();
  readonly #storage: PlatformStorage | undefined;
  /** Set once the user changes the rail, so a kept state read later never undoes the change. */
  #railTouched = false;
  #keepTimer: ReturnType<typeof setTimeout> | undefined;

  /** `storage` keeps the rail's state between visits; without it the rail starts closed each time. */
  constructor(storage?: PlatformStorage) {
    this.#storage = storage;
  }

  /** Reads the rail's kept state; until it resolves the rail is closed, so a first visit never flashes it open. */
  async load(): Promise<void> {
    const kept = readKept(await this.#storage?.get(STORAGE_KEY).catch(() => undefined));
    if (kept !== undefined && !this.#railTouched) this.#set({ rail: kept });
  }

  getState(): ShellLayoutState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Calls `listener` each time `deselect` closes a selection's inspector, so the app clears its own selection too. */
  onDeselect(listener: () => void): () => void {
    this.#deselected.add(listener);
    return () => this.#deselected.delete(listener);
  }

  /** Opens or closes the rail. */
  toggleRail(): void {
    this.#setRail({ open: !this.#state.rail.open });
  }

  /** Opens or closes the rail (`open`). */
  setRailOpen(open: boolean): void {
    if (open !== this.#state.rail.open) this.#setRail({ open });
  }

  /** Resizes the rail to `width`, within its limits. */
  setRailWidth(width: number): void {
    const next = clampRailWidth(width);
    if (next !== this.#state.rail.width) this.#setRail({ width: next });
  }

  /** Selects the rail's `tab`. */
  setRailTab(tab: string): void {
    if (tab !== this.#state.rail.tab) this.#setRail({ tab });
  }

  /** Turns focus mode on or off. */
  toggleFocus(): void {
    this.#set({ focus: !this.#state.focus });
  }

  /** Turns focus mode off, if it is on. */
  leaveFocus(): void {
    if (this.#state.focus) this.#set({ focus: false });
  }

  /** Shows `content` in the inspector, replacing what it showed. */
  openInspector(content: InspectorContent): void {
    const now = this.#state.inspector;
    if (now?.id !== content.id || now.selection !== content.selection) this.#set({ inspector: content });
  }

  /** Closes the inspector. */
  closeInspector(): void {
    if (this.#state.inspector !== undefined) this.#set({ inspector: undefined });
  }

  /** A selection was cleared: closes the inspector if a selection opened it, and leaves a panel opened on purpose. */
  deselect(): void {
    if (this.#state.inspector?.selection !== true) return;
    this.#set({ inspector: undefined });
    for (const listener of [...this.#deselected]) listener();
  }

  /**
   * Escape's step for focus mode: clears a selection first, whichever was opened first, then leaves
   * focus mode (R16: selection, then focus mode).
   */
  escapeFocus(): void {
    if (this.#state.inspector?.selection === true) this.deselect();
    else this.leaveFocus();
  }

  #setRail(change: Partial<ShellLayoutState["rail"]>): void {
    this.#railTouched = true;
    this.#set({ rail: { ...this.#state.rail, ...change } });
    const storage = this.#storage;
    if (storage === undefined) return;
    clearTimeout(this.#keepTimer);
    this.#keepTimer = setTimeout(() => {
      storage.set(STORAGE_KEY, this.#state.rail).catch(() => {
        // Not kept: the rail starts closed on the next visit, which is the default anyway.
      });
    }, KEEP_AFTER_MS);
  }

  #set(change: Partial<ShellLayoutState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}
