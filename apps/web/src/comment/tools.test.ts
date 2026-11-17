import { describe, expect, it } from "vitest";
import type { Mark, Point } from "./marks.ts";
import { type DrawHost, dragTool, inkTool, type PointerInput, placedMark, polyTool, textTool } from "./tools.ts";

/** A host over page 0 at 1 CSS px per point, recording what the tools draw and make. */
function fakeHost() {
  const made: (Mark | undefined)[] = [];
  const drawn: (Mark | undefined)[] = [];
  const timers: (() => void)[] = [];
  const host: DrawHost = {
    at: ({ clientX, clientY }) => ({ page: 0, point: [clientX, clientY] }),
    text: () => ({
      runs: [{ glyphRange: [0, 2] }],
      quads: [10, 22, 20, 22, 20, 10, 10, 10, 20, 22, 30, 22, 30, 10, 20, 10],
    }),
    whenText: (page) => Promise.resolve(host.text(page)),
    draw: (_page, mark) => drawn.push(mark),
    finish: (_page, mark) => made.push(mark),
    later: (run) => {
      timers.push(run);
      return () => timers.splice(timers.indexOf(run), 1);
    },
  };
  return { host, made, drawn, timers };
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

  it("joins ink strokes drawn within a second into one drawing, made when the second passes", () => {
    const { host, made, timers } = fakeHost();
    const tool = inkTool(host);
    dragThrough(tool as never, [
      [0, 0],
      [10, 10],
    ]);
    dragThrough(tool as never, [
      [20, 0],
      [30, 10],
    ]);
    expect(made).toEqual([]);
    timers[timers.length - 1]?.();
    expect(made).toEqual([
      {
        type: "ink",
        strokes: [
          [
            [0, 0],
            [10, 10],
          ],
          [
            [20, 0],
            [30, 10],
          ],
        ],
      },
    ]);
  });

  it("drops a pending ink drawing on cancel", () => {
    const { host, made } = fakeHost();
    const tool = inkTool(host);
    dragThrough(tool as never, [
      [0, 0],
      [10, 10],
    ]);
    tool.cancel();
    expect(made).toEqual([undefined]);
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
});
