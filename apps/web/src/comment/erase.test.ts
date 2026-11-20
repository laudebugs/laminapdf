import { describe, expect, it } from "vitest";
import { boxInside, pointsOf, strokesHit } from "./erase.ts";
import type { Point } from "./marks.ts";

describe("strokesHit", () => {
  const strokes: Point[][] = [
    [
      [10, 10],
      [100, 10],
    ],
    [
      [10, 50],
      [100, 50],
    ],
    [[200, 200]],
  ];

  it("takes the strokes a drag across passes near, and a tap on a dot", () => {
    const across: Point[] = [
      [50, 0],
      [50, 30],
    ];
    expect([strokesHit(strokes, across), strokesHit(strokes, [[203, 198]])]).toEqual([[0], [2]]);
  });

  it("takes none when the path stays clear", () => {
    expect(
      strokesHit(strokes, [
        [300, 300],
        [320, 320],
      ]),
    ).toEqual([]);
  });
});

describe("boxInside", () => {
  const ring: Point[] = [
    [0, 0],
    [100, 0],
    [100, 100],
    [0, 100],
  ];

  it("holds a box wholly inside the loop, not one across its edge", () => {
    expect([boxInside([10, 10, 50, 50], ring), boxInside([80, 80, 120, 120], ring)]).toEqual([true, false]);
  });
});

describe("pointsOf", () => {
  it("reads a flat list as points", () => {
    expect(pointsOf([1, 2, 3, 4])).toEqual([
      [1, 2],
      [3, 4],
    ]);
  });
});
