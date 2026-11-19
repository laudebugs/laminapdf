// Which pointer draws in Comment mode on a phone or tablet (C08, C09): palm rejection by pointer
// type. A pen always draws; a finger draws only with Draw with finger on, and otherwise pans and
// pinches as the viewer's gestures do; a touch while a pen is drawing is a resting hand, and
// ignored; so is a large contact while a finger draws. A mouse draws as on the desktop.

import type { MarkTool } from "./tools.ts";

/** What a press is for. */
export type PointerUse = "draw" | "pan" | "swallow";

/** The parts of a pointer event the rules read. */
export interface RuledPointer {
  readonly pointerId: number;
  readonly pointerType: string;
  readonly width: number;
  readonly height: number;
}

/** A contact wider or taller than this, CSS px, is a palm or the side of a hand, not a fingertip. */
export const LARGE_CONTACT = 40;

/** The rules' settings and what they have seen. */
export interface PointerRulesOptions {
  /** Whether a finger draws (Draw with finger). */
  readonly drawWithFinger: () => boolean;
  /** Called on the first pen press: the tablet then shows the tool shelf (C09). */
  readonly penSeen?: () => void;
}

/** Palm rejection by pointer type, for one Comment mode. */
export class PointerRules {
  readonly #options: PointerRulesOptions;
  /** The pens pressed now. */
  readonly #pens = new Set<number>();
  #seen = false;

  constructor(options: PointerRulesOptions) {
    this.#options = options;
  }

  /** What a press is for: drawing, the viewer's pan and pinch, or nothing at all. */
  use(e: RuledPointer): PointerUse {
    if (e.pointerType === "pen") return "draw";
    if (e.pointerType !== "touch") return "draw";
    if (this.#pens.size > 0) return "swallow";
    if (!this.#options.drawWithFinger()) return "pan";
    return Math.max(e.width, e.height) > LARGE_CONTACT ? "swallow" : "draw";
  }

  /** A press the rules let draw has started. */
  pressed(e: RuledPointer): void {
    if (e.pointerType !== "pen") return;
    this.#pens.add(e.pointerId);
    if (!this.#seen) {
      this.#seen = true;
      this.#options.penSeen?.();
    }
  }

  /** A press has ended. */
  released(e: RuledPointer): void {
    this.#pens.delete(e.pointerId);
  }
}

/** `tool` with the rules in front of it: it sees only the presses that draw. */
export function ruled(tool: MarkTool, rules: PointerRules): MarkTool {
  return {
    ...tool,
    pointerDown(event, host) {
      const use = rules.use(event);
      if (use === "swallow") event.preventDefault();
      if (use !== "draw") return false;
      const took = tool.pointerDown?.(event, host) ?? false;
      if (took) rules.pressed(event);
      return took;
    },
    pointerUp(event, host) {
      rules.released(event);
      tool.pointerUp?.(event, host);
    },
  };
}
