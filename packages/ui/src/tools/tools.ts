// Interaction modes (E1 plan §5, full-editor §6): how pointer input on the page is read. Each is a
// tool module registered by name: its cursor, the viewer layer it draws into, and its pointer
// handlers; the reader machine's `set-mode` takes any registered name and knows nothing else
// about it. The names are the keys of `InteractionModes`, which later rounds extend by
// declaration merging, so a new tool is one interface entry and one `register` call:
//
//   declare module "@laminapdf/ui" {
//     interface InteractionModes {
//       annotate: true;
//     }
//   }
//   tools.register({ name: "annotate", cursor: "crosshair", layer: "annot", selectsText: false, … });
//
// Shell modes (Read, Comment, Organise…) are another registry (shell/modes.ts): a shell mode owns a
// toolbar, and its tools are interaction modes.

import type { ExternalStore } from "../state/hooks.js";

/**
 * The interaction modes, by name. `select`: choose objects on the page (text, until an object
 * layer exists); `hand`: drag to scroll; `textSelect`: select text only; `link`: select a link,
 * or draw one (O12). Later rounds add `annotate`, `formFill`, `formDesign`, `contentEdit`,
 * `redact` and `measure`.
 */
export interface InteractionModes {
  select: true;
  hand: true;
  textSelect: true;
  link: true;
}

/** An interaction mode's name: any key of `InteractionModes`. */
export type InteractionMode = keyof InteractionModes;

/** What a tool drives: the viewer's scroll container. */
export interface ToolHost {
  /** The element the pointer lands on: the viewer's scroll container. */
  readonly element: HTMLElement;
  /** Scrolls the view by this many CSS px. */
  scrollBy(dx: number, dy: number): void;
}

/** A tool module: one interaction mode. */
export interface ToolModule {
  readonly name: InteractionMode;
  /** The CSS cursor over the page while it is on; empty for the page's own (text over text). */
  readonly cursor: string;
  /** The viewer layer it draws into (`text`, `annot`, `highlight`), if any. */
  readonly layer?: string;
  /** Whether a drag over text selects it while this tool is on. */
  readonly selectsText: boolean;
  /** A press on the page; return true to take the pointer until it is released. */
  pointerDown?(event: PointerEvent, host: ToolHost): boolean;
  /** A move while this tool holds the pointer (its `pointerDown` returned true). */
  pointerMove?(event: PointerEvent, host: ToolHost): void;
  /** The release that ends this tool's hold on the pointer. */
  pointerUp?(event: PointerEvent, host: ToolHost): void;
}

/** The registry's state: the tools by name and the one on. */
export interface ToolsState {
  readonly tools: ReadonlyMap<InteractionMode, ToolModule>;
  readonly current: InteractionMode;
}

/** Selecting what is on the page: text, for now, as the reader always has. */
export const SELECT_TOOL: ToolModule = { name: "select", cursor: "", selectsText: true };

/** Selecting text only. */
export const TEXT_SELECT_TOOL: ToolModule = { name: "textSelect", cursor: "text", selectsText: true };

/** Drags scroll the page, as a hand moves paper; nothing is selected. */
export function handTool(): ToolModule {
  let last: { x: number; y: number } | undefined;
  return {
    name: "hand",
    cursor: "grab",
    selectsText: false,
    pointerDown(event) {
      if (event.button !== 0) return false;
      last = { x: event.clientX, y: event.clientY };
      return true;
    },
    pointerMove(event, host) {
      if (last === undefined) return;
      host.scrollBy(last.x - event.clientX, last.y - event.clientY);
      last = { x: event.clientX, y: event.clientY };
    },
    pointerUp() {
      last = undefined;
    },
  };
}

/** The interaction modes registered, and the one on (`select` at first). */
export class ToolRegistry implements ExternalStore<ToolsState> {
  #state: ToolsState;
  readonly #listeners = new Set<() => void>();

  /** A registry holding the built-in tools: select, hand and text select. */
  constructor(tools: readonly ToolModule[] = [SELECT_TOOL, handTool(), TEXT_SELECT_TOOL]) {
    this.#state = { tools: new Map(tools.map((tool) => [tool.name, tool])), current: "select" };
  }

  getState = (): ToolsState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Adds `tool`; returns the function that removes it (switching back to `select` if it was on). */
  register(tool: ToolModule): () => void {
    if (this.#state.tools.has(tool.name)) throw new Error(`tool ${tool.name} is already registered`);
    this.#set({ tools: new Map([...this.#state.tools, [tool.name, tool]]) });
    return () => {
      const tools = new Map(this.#state.tools);
      tools.delete(tool.name);
      this.#set({ tools, current: this.#state.current === tool.name ? "select" : this.#state.current });
    };
  }

  /** Whether `name` is registered. */
  has(name: InteractionMode): boolean {
    return this.#state.tools.has(name);
  }

  /** The tool on now. */
  current(): ToolModule | undefined {
    return this.#state.tools.get(this.#state.current);
  }

  /** Turns `name` on; an unregistered name is ignored. Returns whether it is on now. */
  use(name: InteractionMode): boolean {
    if (!this.has(name)) return false;
    if (name !== this.#state.current) this.#set({ current: name });
    return true;
  }

  #set(next: Partial<ToolsState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of [...this.#listeners]) listener();
  }
}

/**
 * Passes pointer input on `host.element` to the tool on in `registry`, and shows its cursor; the
 * element's `data-tool` names it, so styles can follow (a tool that selects no text turns the text
 * layer's selection off). Returns the function that stops.
 */
export function attachTools(registry: ToolRegistry, host: ToolHost): () => void {
  const { element } = host;
  let captured: { tool: ToolModule; pointer: number } | undefined;
  const show = () => {
    const tool = registry.current();
    element.dataset.tool = tool?.name ?? "";
    element.style.cursor = tool?.cursor ?? "";
    if (tool?.selectsText === false) element.dataset.noTextSelection = "";
    else delete element.dataset.noTextSelection;
  };
  const down = (event: PointerEvent) => {
    const tool = registry.current();
    if (tool?.pointerDown?.(event, host) !== true) return;
    captured = { tool, pointer: event.pointerId };
    element.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  };
  const move = (event: PointerEvent) => {
    if (captured?.pointer === event.pointerId) captured.tool.pointerMove?.(event, host);
  };
  const up = (event: PointerEvent) => {
    if (captured?.pointer !== event.pointerId) return;
    captured.tool.pointerUp?.(event, host);
    captured = undefined;
  };
  show();
  const stop = registry.subscribe(show);
  element.addEventListener("pointerdown", down);
  element.addEventListener("pointermove", move);
  element.addEventListener("pointerup", up);
  element.addEventListener("pointercancel", up);
  return () => {
    stop();
    element.removeEventListener("pointerdown", down);
    element.removeEventListener("pointermove", move);
    element.removeEventListener("pointerup", up);
    element.removeEventListener("pointercancel", up);
    delete element.dataset.tool;
    delete element.dataset.noTextSelection;
    element.style.cursor = "";
  };
}
