import { describe, expect, it } from "vitest";
import type { Mark, Point } from "./marks.ts";
import {
  type DrawHost,
  dragTool,
  inkTool,
  type PointerInput,
  placedMark,
  polyTool,
  textTool,
  typeTool,
} from "./tools.ts";

/** A host over page 0 at 1 CSS px per point, recording what the tools draw and make. */
function fakeHost() {
  const made: (Mark | undefined)[] = [];
  const drawn: (Mark | undefined)[] = [];
  const timers: (() => void)[] = [];
  const opened: { page: number; kind: string; from: Point; to: Point }[] = [];
  /** Ink steps: a new drawing, or strokes joined to one, in order. */
  const inked: string[] = [];
  let next = 0;
  let typing = false;
  const host: DrawHost = {
    at: ({ clientX, clientY }) => ({ page: 0, point: [clientX, clientY] }),
    text: () => ({
      runs: [{ glyphRange: [0, 2] }],
      quads: [10, 22, 20, 22, 20, 10, 10, 10, 20, 22, 30, 22, 30, 10, 20, 10],
    }),
    whenText: (page) => Promise.resolve(host.text(page)),
    draw: (_page, mark) => drawn.push(mark),
    finish: (_page, mark) => made.push(mark),
    inkStroke: async (_page, mark, into) => {
      if (into === undefined) {
        next += 1;
        inked.push(`make d${next} ${JSON.stringify(mark)}`);
        return `d${next}`;
      }
      const id = await into;
      if (id !== undefined) inked.push(`join ${id} ${JSON.stringify(mark)}`);
      return id;
    },
    later: (run) => {
      timers.push(run);
      return () => timers.splice(timers.indexOf(run), 1);
    },
    openText: (page, kind, from, to) => {
      opened.push({ page, kind, from, to });
      typing = true;
    },
    typing: () => typing,
    stampSize: () => undefined,
    placeStamp: () => undefined,
    attachAt: () => undefined,
  };
  return { host, made, drawn, timers, opened, inked, stopTyping: () => (typing = false) };
}

/** A primary-button pointer at (`x`, `y`). */
const pointer = (x: number, y: number, extra: Partial<PointerInput> = {}) =>
  ({ clientX: x, clientY: y, button: 0, shiftKey: false, timeStamp: 0, ...extra }) as PointerEvent;

const NO_HOST = undefined as never;

/** Presses at the first point, moves through the rest and releases at the last. */
function dragThrough(tool: ReturnType<typeof dragTool>, points: readonly Point[], extra: Partial<PointerInput> = {}) {
  const [first, ...rest] = points;
  if (first === undefined) return;
  tool.pointerDown?.(pointer(...first, extra), NO_HOST);
  for (const p of rest) tool.pointerMove?.(pointer(...p, extra), NO_HOST);
  const last = rest[rest.length - 1] ?? first;
  tool.pointerUp?.(pointer(...last, extra), NO_HOST);
}

describe("the comment tools", () => {
  it("draws a rectangle from a drag, and a square with Shift", () => {
    const { host, made } = fakeHost();
    const tool = dragTool("rectangle", host);
    dragThrough(tool, [
      [10, 10],
      [50, 30],
    ]);
    dragThrough(
      tool,
      [
        [10, 10],
        [50, 30],
      ],
      { shiftKey: true },
    );
    expect(made).toEqual([
      { type: "shape", kind: "square", box: [10, 10, 50, 30] },
      { type: "shape", kind: "square", box: [10, 10, 50, 50] },
    ]);
  });

  it("makes nothing from a click with a shape tool", () => {
    const { host, made } = fakeHost();
    dragThrough(dragTool("ellipse", host), [[10, 10]]);
    expect(made).toEqual([undefined]);
  });

  it("highlights the glyphs a drag crosses", () => {
    const { host, made } = fakeHost();
    const tool = textTool("highlight", host);
    tool.pointerDown?.(pointer(12, 15), NO_HOST);
    tool.pointerMove?.(pointer(28, 15), NO_HOST);
    tool.pointerUp?.(pointer(28, 15), NO_HOST);
    expect(made).toEqual([{ type: "markup", kind: "highlight", lines: [[10, 10, 30, 22]] }]);
  });

  it("makes each ink stroke a step: the first a drawing, one within a second joined to it, a later one a new drawing", async () => {
    const { host, inked, timers } = fakeHost();
    const tool = inkTool(host);
    const stroke = (from: Point, to: Point) => dragThrough(tool as never, [from, to]);
    stroke([0, 0], [10, 10]);
    stroke([20, 0], [30, 10]);
    await Promise.resolve();
    timers[timers.length - 1]?.();
    stroke([40, 0], [50, 10]);
    await Promise.resolve();
    expect(inked).toEqual([
      'make d1 {"type":"ink","strokes":[[[0,0],[10,10]]]}',
      'join d1 {"type":"ink","strokes":[[[20,0],[30,10]]]}',
      'make d2 {"type":"ink","strokes":[[[40,0],[50,10]]]}',
    ]);
  });

  it("keeps a pen's pressure at each point, and none for a mouse or a finger", () => {
    const { host, inked } = fakeHost();
    const tool = inkTool(host);
    dragThrough(
      tool as never,
      [
        [0, 0],
        [10, 10],
      ],
      { pointerType: "pen", pressure: 0.8 },
    );
    expect(inked).toEqual(['make d1 {"type":"ink","strokes":[[[0,0],[10,10]]],"pressures":[[0.8,0.8]]}']);
  });

  it("drops a stroke still being drawn on cancel", () => {
    const { host, made, inked } = fakeHost();
    const tool = inkTool(host);
    tool.pointerDown?.(pointer(0, 0), NO_HOST);
    tool.pointerMove?.(pointer(10, 10), NO_HOST);
    tool.cancel();
    expect([made, inked]).toEqual([[undefined], []]);
  });

  it("ends a polygon with a second click on its last corner, and refuses one with two corners", () => {
    const { host, made } = fakeHost();
    const tool = polyTool("polygon", host);
    const tap = (x: number, y: number, t: number) => {
      tool.pointerDown?.(pointer(x, y, { timeStamp: t }), NO_HOST);
      tool.pointerUp?.(pointer(x, y, { timeStamp: t }), NO_HOST);
    };
    tap(0, 0, 0);
    tap(40, 0, 1000);
    tap(20, 30, 2000);
    tap(20, 30, 2100);
    tap(0, 0, 5000);
    tap(40, 0, 6000);
    tool.done();
    expect(made).toEqual([
      {
        type: "poly",
        kind: "polygon",
        vertices: [
          [0, 0],
          [40, 0],
          [20, 30],
        ],
      },
      undefined,
    ]);
  });

  it("places a default mark in the middle of the page from the keyboard", () => {
    expect(placedMark("rectangle", [100, 100], undefined)).toEqual({
      type: "shape",
      kind: "square",
      box: [52, 68, 148, 132],
    });
    expect(placedMark("highlight", [100, 100], undefined)).toBeUndefined();
  });

  it("opens a text box for typing from a press and release, and lets a press only end the box open", () => {
    const { host, opened, stopTyping } = fakeHost();
    const tool = typeTool("textBox", host);
    dragThrough(tool, [
      [10, 10],
      [120, 40],
    ]);
    const pressed = tool.pointerDown?.(pointer(200, 200), NO_HOST);
    stopTyping();
    dragThrough(tool, [[200, 200]]);
    expect([opened, pressed]).toEqual([
      [
        { page: 0, kind: "textBox", from: [10, 10], to: [120, 40] },
        { page: 0, kind: "textBox", from: [200, 200], to: [200, 200] },
      ],
      false,
    ]);
  });
});
