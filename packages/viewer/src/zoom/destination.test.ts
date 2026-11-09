import type { Target } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { CSS_PX_PER_PT } from "../layout/page-layout.js";
import { destinationView } from "./destination.js";

const A4 = { width: 600, height: 800 };
const VIEWPORT = { width: 1000, height: 832 };
const GAP = 16;

const go = (view: Target["view"], rotation: 0 | 90 | 180 | 270 = 0) =>
  destinationView({ page: 3, view }, A4, rotation, VIEWPORT, GAP);

describe("destinationView", () => {
  it("puts an XYZ point at the top-left at its zoom", () => {
    expect(go({ kind: "at", x: 150, y: 200, zoom: 2 })).toEqual({ zoom: 200, page: 3, x: 0.25, y: 0.25 });
  });

  it("keeps the zoom and the horizontal scroll an XYZ leaves unset", () => {
    expect(go({ kind: "at", y: 400 })).toEqual({ zoom: undefined, page: 3, x: undefined, y: 0.5 });
  });

  it("fits the page", () => {
    expect(go({ kind: "fit" })).toEqual({ zoom: "fit-page", page: 3, x: undefined, y: undefined });
  });

  it("fits the width from a top", () => {
    expect(go({ kind: "fit-width", y: 80 })).toEqual({ zoom: "fit-width", page: 3, x: undefined, y: 0.1 });
  });

  it("fits the height from a left", () => {
    const view = go({ kind: "fit-height", x: 300 });
    expect(view.x).toBe(0.5);
    expect(view.zoom).toBeCloseTo(((832 - 32) / 800 / CSS_PX_PER_PT) * 100);
  });

  it("fits a rectangle in the room left by the gaps", () => {
    const view = go({ kind: "fit-rect", x0: 0, y0: 400, x1: 300, y1: 600 });
    expect([view.x, view.y]).toEqual([0, 0.5]);
    // 300 × 200 pt in 968 × 800 CSS px: the height limits it.
    expect(view.zoom).toBeCloseTo((Math.min(968 / 300, 800 / 200) / CSS_PX_PER_PT) * 100);
  });

  it("clamps a point off the page onto it", () => {
    expect(go({ kind: "at", x: -10, y: 1e6 })).toMatchObject({ x: 0, y: 1 });
  });

  it.each([
    [90, { x: 0.75, y: 0.25 }],
    [180, { x: 0.75, y: 0.75 }],
    [270, { x: 0.25, y: 0.75 }],
  ] as const)("turns the point with a view rotated %i°", (rotation, at) => {
    expect(go({ kind: "at", x: 150, y: 200 }, rotation)).toMatchObject(at);
  });

  it("measures a fit-height on a turned view by the page's width", () => {
    const view = go({ kind: "fit-height" }, 90);
    expect(view.zoom).toBeCloseTo(((832 - 32) / 600 / CSS_PX_PER_PT) * 100);
  });

  it("orders a turned rectangle's corners", () => {
    expect(go({ kind: "fit-rect", x0: 0, y0: 0, x1: 300, y1: 400 }, 180)).toMatchObject({ x: 0.5, y: 0.5 });
  });
});
