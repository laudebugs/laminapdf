// Reading positions in the app: a stored view is checked before the viewer gets it, the home
// screen's page is clamped to the page count, and the recorder saves a second after the view
// stops, at once when the page is hidden, and when it stops.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type KeptView,
  type PageVisibility,
  parseView,
  recordPosition,
  SAVE_AFTER_MS,
  stoppedAt,
} from "./positions.ts";

const VIEW: KeptView = { point: { page: 11, x: 0, y: 0.3 }, zoom: 150, rotation: 0 };

describe("a kept view", () => {
  it("is taken as the viewer gave it", () => {
    expect(parseView(VIEW)).toEqual(VIEW);
    expect(parseView({ ...VIEW, zoom: "fit-width", rotation: 270 })).toEqual({
      ...VIEW,
      zoom: "fit-width",
      rotation: 270,
    });
  });

  it("is refused when any part isn't a position the viewer could have written", () => {
    for (const bad of [
      undefined,
      "page 12",
      { ...VIEW, point: { page: -1, x: 0, y: 0 } },
      { ...VIEW, point: { page: 1.5, x: 0, y: 0 } },
      { ...VIEW, point: { page: 1, x: Number.NaN, y: 0 } },
      { ...VIEW, zoom: 5000 },
      { ...VIEW, zoom: "huge" },
      { ...VIEW, rotation: 45 },
    ]) {
      expect(parseView(bad)).toBeUndefined();
    }
  });

  it("clamps a point beside the page onto its edge", () => {
    expect(parseView({ ...VIEW, point: { page: 3, x: -0.2, y: 1.4 } })?.point).toEqual({ page: 3, x: 0, y: 1 });
  });

  it("gives the home screen its page, within the page count", () => {
    expect(stoppedAt({ view: VIEW, pageCount: 40, readAt: 1 })).toEqual({ page: 11, pageCount: 40 });
    expect(stoppedAt({ view: VIEW, pageCount: 5, readAt: 1 })).toEqual({ page: 4, pageCount: 5 });
    expect(stoppedAt({ view: "damaged", pageCount: 5, readAt: 1 })).toBeUndefined();
    expect(stoppedAt(undefined)).toBeUndefined();
  });
});

/** A viewer whose position the test moves, and a page whose visibility it sets. */
function fakes() {
  const listeners = new Set<() => void>();
  let position: KeptView | undefined = VIEW;
  const viewer = {
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    viewPosition: () => position,
  };
  const pageListeners = new Set<() => void>();
  const page: PageVisibility & { visibilityState: DocumentVisibilityState } = {
    visibilityState: "visible",
    addEventListener: (_type, listener) => void pageListeners.add(listener),
    removeEventListener: (_type, listener) => void pageListeners.delete(listener),
  };
  return {
    viewer,
    page,
    moveTo(page: number) {
      position = { ...VIEW, point: { ...VIEW.point, page } };
      for (const listener of listeners) listener();
    },
    hide() {
      page.visibilityState = "hidden";
      for (const listener of pageListeners) listener();
    },
    listening: () => listeners.size + pageListeners.size,
  };
}

describe("the position recorder", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("saves a second after the view stops changing, once", () => {
    const { viewer, page, moveTo } = fakes();
    const saved: number[] = [];
    recordPosition(viewer, (view) => saved.push(view.point.page), page);
    moveTo(3);
    vi.advanceTimersByTime(SAVE_AFTER_MS / 2);
    moveTo(4);
    vi.advanceTimersByTime(SAVE_AFTER_MS - 1);
    expect(saved).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(saved).toEqual([4]);
  });

  it("saves at once when the page is hidden", () => {
    const { viewer, page, moveTo, hide } = fakes();
    const saved: number[] = [];
    recordPosition(viewer, (view) => saved.push(view.point.page), page);
    moveTo(7);
    hide();
    expect(saved).toEqual([7]);
  });

  it("saves at once on flush and goes on recording", () => {
    const { viewer, page, moveTo } = fakes();
    const saved: number[] = [];
    const recorder = recordPosition(viewer, (view) => saved.push(view.point.page), page);
    moveTo(5);
    recorder.flush();
    moveTo(6);
    vi.advanceTimersByTime(SAVE_AFTER_MS);
    expect(saved).toEqual([5, 6]);
  });

  it("saves a last time when it stops, and listens to nothing after", () => {
    const { viewer, page, moveTo, listening } = fakes();
    const saved: number[] = [];
    const recorder = recordPosition(viewer, (view) => saved.push(view.point.page), page);
    moveTo(9);
    recorder.stop();
    expect(saved).toEqual([9]);
    expect(listening()).toBe(0);
    vi.advanceTimersByTime(SAVE_AFTER_MS * 2);
    expect(saved).toEqual([9]);
  });
});
