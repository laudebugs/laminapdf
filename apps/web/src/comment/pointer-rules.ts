// Which pointer draws in Comment mode on a phone or tablet (C08, C09): palm rejection by pointer
// type. A pen always draws; a finger draws only with Draw with finger on, and otherwise pans and
// pinches as the viewer's gestures do; a touch while a pen is drawing is a resting hand, and
// ignored; so is a large contact while a finger draws. A second finger while one draws means
// two fingers to pan or pinch: the stroke is dropped and both go to the viewer. A mouse draws as
// on the desktop.

import type { MarkTool } from "./tools.ts";

/** What a press is for; `handover` gives it, and the finger drawing, to the viewer's gestures. */
export type PointerUse = "draw" | "pan" | "swallow" | "handover";

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
  /** Gives a finger's press back to the viewer's gestures, which hadn't seen it. */
  readonly handOver?: (down: RuledPointer) => void;
}

/** Palm rejection by pointer type, for one Comment mode. */
export class PointerRules {
  readonly #options: PointerRulesOptions;
  /** The pens pressed now. */
  readonly #pens = new Set<number>();
  /** The fingers drawing now, by their presses. */
  readonly #fingers = new Map<number, RuledPointer>();
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
    if (Math.max(e.width, e.height) > LARGE_CONTACT) return "swallow";
    return this.#fingers.size > 0 ? "handover" : "draw";
  }

  /** A press the rules let draw has started. */
  pressed(e: RuledPointer): void {
    if (e.pointerType === "touch") this.#fingers.set(e.pointerId, e);
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
    this.#fingers.delete(e.pointerId);
  }

  /** Gives the fingers drawing to the viewer's gestures, and forgets them. */
  handOver(): void {
    for (const down of this.#fingers.values()) this.#options.handOver?.(down);
    this.#fingers.clear();
  }
}

/** `tool` with the rules in front of it: it sees only the presses that draw. */
export function ruled(tool: MarkTool, rules: PointerRules): MarkTool {
  return {
    ...tool,
    pointerDown(event, host) {
      const use = rules.use(event);
      if (use === "swallow") event.preventDefault();
      if (use === "handover") {
        // Two fingers: the stroke the first began is dropped, and both pan or pinch.
        tool.cancel();
        rules.handOver();
      }
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
