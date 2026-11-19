import { describe, expect, it } from "vitest";
import { PointerRules, type RuledPointer, ruled } from "./pointer-rules.ts";
import type { MarkTool } from "./tools.ts";

const pen: RuledPointer = { pointerId: 1, pointerType: "pen", width: 1, height: 1 };
const finger: RuledPointer = { pointerId: 2, pointerType: "touch", width: 12, height: 12 };
const palm: RuledPointer = { pointerId: 3, pointerType: "touch", width: 70, height: 90 };
const mouse: RuledPointer = { pointerId: 4, pointerType: "mouse", width: 1, height: 1 };

describe("PointerRules", () => {
  it("lets a pen and a mouse draw, and a finger pan, with Draw with finger off", () => {
    const rules = new PointerRules({ drawWithFinger: () => false });
    expect([rules.use(pen), rules.use(mouse), rules.use(finger)]).toEqual(["draw", "draw", "pan"]);
  });

  it("lets a finger draw with Draw with finger on, but not a palm", () => {
    const rules = new PointerRules({ drawWithFinger: () => true });
    expect([rules.use(finger), rules.use(palm)]).toEqual(["draw", "swallow"]);
  });

  it("ignores every touch while a pen draws, and says the first time a pen is seen", () => {
    const seen: string[] = [];
    const rules = new PointerRules({ drawWithFinger: () => true, penSeen: () => seen.push("pen") });
    rules.pressed(pen);
    const during = rules.use(finger);
    rules.released(pen);
    rules.pressed(pen);
    expect([during, seen]).toEqual(["swallow", ["pen"]]);
  });
});

describe("ruled", () => {
  /** A tool that takes every press and records it. */
  function recording(log: string[]): MarkTool {
    return {
      name: "ink",
      cursor: "crosshair",
      selectsText: false,
      pointerDown: (e) => {
        log.push(`down ${e.pointerType}`);
        return true;
      },
      pointerUp: (e) => {
        log.push(`up ${e.pointerType}`);
      },
      done: () => {},
      cancel: () => {},
    };
  }

  /** A pointer event as the tool registry passes it, with `preventDefault` recorded. */
  function event(p: RuledPointer) {
    const e = { ...p, prevented: false, preventDefault: () => (e.prevented = true) };
    return e;
  }

  it("passes the tool only the presses that draw, and swallows a palm during a pen stroke", () => {
    const log: string[] = [];
    const tool = ruled(recording(log), new PointerRules({ drawWithFinger: () => false }));
    const host = {} as never;
    const down = (p: RuledPointer) => {
      const e = event(p);
      const took = tool.pointerDown?.(e as never, host);
      return [took, e.prevented];
    };
    const penDown = down(pen);
    const palmDuring = down(palm);
    tool.pointerUp?.(event(pen) as never, host);
    const fingerAfter = down(finger);
    expect([penDown, palmDuring, fingerAfter, log]).toEqual([
      [true, false],
      [false, true],
      [false, false],
      ["down pen", "up pen"],
    ]);
  });
});

describe("a second finger", () => {
  it("drops the stroke the first began and gives both to the viewer", () => {
    const given: number[] = [];
    const log: string[] = [];
    const rules = new PointerRules({ drawWithFinger: () => true, handOver: (down) => given.push(down.pointerId) });
    const tool = ruled(
      {
        name: "ink",
        cursor: "crosshair",
        selectsText: false,
        pointerDown: () => true,
        done: () => {},
        cancel: () => log.push("cancel"),
      },
      rules,
    );
    const press = (p: RuledPointer) => tool.pointerDown?.({ ...p, preventDefault: () => {} } as never, {} as never);
    const first = press(finger);
    const second = press({ ...finger, pointerId: 9 });
    expect([first, second, log, given]).toEqual([true, false, ["cancel"], [2]]);
  });
});
