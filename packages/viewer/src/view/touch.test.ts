import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DOUBLE_TAP,
  isDoubleTap,
  LONG_PRESS_MS,
  momentumStep,
  pinchPercent,
  releaseVelocity,
  SLOP,
  TouchGestures,
  type TouchHost,
} from "./touch.ts";

afterEach(() => {
  vi.useRealTimers();
});

describe("the gesture maths", () => {
  it("counts a second tap as a double tap only soon enough and close enough", () => {
    const first = { x: 100, y: 100, t: 0 };
    expect([
      isDoubleTap(first, { x: 110, y: 105, t: 200 }),
      isDoubleTap(first, { x: 100, y: 100, t: DOUBLE_TAP.ms + 1 }),
      isDoubleTap(first, { x: 100 + DOUBLE_TAP.distance + 1, y: 100, t: 100 }),
    ]).toEqual([true, false, false]);
  });

  it("scales the zoom by how far the fingers moved apart", () => {
    expect([pinchPercent(100, 100, 250), pinchPercent(150, 200, 100), pinchPercent(100, 0, 50)]).toEqual([
      250, 75, 100,
    ]);
  });

  it("takes a flick's speed from its last 100 ms, not the whole pan", () => {
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 0, y: 10, t: 290 },
      { x: 0, y: 20, t: 350 },
      { x: 0, y: 70, t: 400 },
    ];
    expect(releaseVelocity(samples)).toEqual({ x: 0, y: 1 });
    expect(releaseVelocity([])).toEqual({ x: 0, y: 0 });
  });

  it("slows momentum down frame by frame until it stops", () => {
    let v = { x: 0, y: 2 };
    let travelled = 0;
    let frames = 0;
    for (let step = momentumStep(v, 16); step !== undefined; step = momentumStep(v, 16)) {
      travelled += step.move.y;
      v = step.v;
      frames += 1;
    }
    // The integral of 2 px/ms decaying with a 325 ms time constant, less the tail below the stop speed.
    expect([Math.round(travelled), frames < 200]).toEqual([644, true]);
  });
});

/** A host over a plain number line: content position = viewport position + scroll, scaled by the zoom. */
function fakeHost() {
  const state = { percent: 100, fit: true, scroll: { x: 0, y: 0 }, zooms: [] as (number | "fit-width")[] };
  const host: TouchHost<{ x: number; y: number }> = {
    percent: () => state.percent,
    fitsWidth: () => state.fit,
    zoom: (to, at) => {
      state.zooms.push(to);
      const next = to === "fit-width" ? 100 : to;
      // Keep the content point under `at` still, as the viewer does.
      const k = next / state.percent;
      state.scroll = { x: (state.scroll.x + at.x) * k - at.x, y: (state.scroll.y + at.y) * k - at.y };
      state.percent = next;
      state.fit = to === "fit-width";
    },
    pointAt: (at) => ({ x: (at.x + state.scroll.x) / state.percent, y: (at.y + state.scroll.y) / state.percent }),
    positionOf: (p) => ({ x: p.x * state.percent - state.scroll.x, y: p.y * state.percent - state.scroll.y }),
    scrollBy: (dx, dy) => {
      state.scroll = { x: state.scroll.x + dx, y: state.scroll.y + dy };
    },
    local: (e) => ({ x: e.clientX, y: e.clientY }),
    frame: () => 0,
    cancelFrame: () => undefined,
    capture: () => undefined,
  };
  return { state, gestures: new TouchGestures(host) };
}

const touch = (type: string, id: number, x: number, y: number, t: number) =>
  ({ type, pointerType: "touch", pointerId: id, clientX: x, clientY: y, timeStamp: t }) as unknown as PointerEvent;

describe("TouchGestures", () => {
  it("keeps the point that started under the fingers under them, as they spread and move together", () => {
    const { state, gestures } = fakeHost();
    gestures.down(touch("pointerdown", 1, 100, 100, 0));
    gestures.down(touch("pointerdown", 2, 200, 100, 0));
    const start = { x: 150 / 100, y: 100 / 100 };
    gestures.move(touch("pointermove", 1, 60, 140, 16));
    gestures.move(touch("pointermove", 2, 300, 140, 16));
    const now = { x: start.x * state.percent - state.scroll.x, y: start.y * state.percent - state.scroll.y };
    expect([state.percent, Math.abs(now.x - 180) < 2, Math.abs(now.y - 140) < 2]).toEqual([240, true, true]);
  });

  it("pans one finger past the slop, and leaves a smaller wobble alone", () => {
    const { state, gestures } = fakeHost();
    gestures.down(touch("pointerdown", 1, 100, 300, 0));
    gestures.move(touch("pointermove", 1, 100, 300 - (SLOP - 1), 10));
    const still = state.scroll.y;
    gestures.move(touch("pointermove", 1, 100, 200, 30));
    expect([still, state.scroll.y]).toEqual([0, 100]);
  });

  it("zooms in on a double tap at the page width, and back to the page width otherwise", () => {
    const { state, gestures } = fakeHost();
    for (const t of [0, 150, 1000, 1150]) {
      gestures.down(touch("pointerdown", 1, 50, 50, t));
      gestures.up(touch("pointerup", 1, 50, 50, t + 50));
    }
    expect(state.zooms).toEqual([200, "fit-width"]);
  });

  it("leaves a finger held still to the browser, so a long press can select text", () => {
    vi.useFakeTimers();
    const { state, gestures } = fakeHost();
    gestures.down(touch("pointerdown", 1, 100, 100, 0));
    vi.advanceTimersByTime(LONG_PRESS_MS + 1);
    gestures.move(touch("pointermove", 1, 100, 300, LONG_PRESS_MS + 50));
    expect(state.scroll).toEqual({ x: 0, y: 0 });
  });

  it("leaves the mouse to the browser, and pans with a pen", () => {
    const { state, gestures } = fakeHost();
    const as = (type: string, pointerType: string, y: number) =>
      ({ ...touch(type, 1, 100, y, 0), pointerType }) as PointerEvent;
    gestures.down(as("pointerdown", "mouse", 300));
    gestures.move(as("pointermove", "mouse", 200));
    const mouse = state.scroll.y;
    gestures.down(as("pointerdown", "pen", 300));
    gestures.move(as("pointermove", "pen", 200));
    expect([mouse, state.scroll.y]).toEqual([0, 100]);
  });

  it("keeps pinching with the first two fingers when a third comes and goes, and pans on from the one left", () => {
    const { state, gestures } = fakeHost();
    gestures.down(touch("pointerdown", 1, 100, 100, 0));
    gestures.down(touch("pointerdown", 2, 200, 100, 0));
    gestures.down(touch("pointerdown", 3, 150, 300, 0));
    gestures.up(touch("pointerup", 3, 150, 300, 10));
    gestures.move(touch("pointermove", 2, 300, 100, 20));
    const zoomed = state.percent;
    gestures.up(touch("pointerup", 2, 300, 100, 30));
    const before = { ...state.scroll };
    gestures.move(touch("pointermove", 1, 100, 60, 40));
    expect([zoomed, state.scroll.x - before.x, state.scroll.y - before.y]).toEqual([200, 0, 40]);
  });

  it("times a double tap from the first tap's lift", () => {
    const { state, gestures } = fakeHost();
    gestures.down(touch("pointerdown", 1, 50, 50, 0));
    gestures.up(touch("pointerup", 1, 50, 50, 200));
    gestures.down(touch("pointerdown", 1, 50, 50, 200 + DOUBLE_TAP.ms));
    gestures.up(touch("pointerup", 1, 50, 50, 250 + DOUBLE_TAP.ms));
    expect(state.zooms).toEqual([200]);
  });
});
