import { describe, expect, it } from "vitest";
import { pageMoves } from "./page-moves.js";

describe("pageMoves", () => {
  it("follows moved pages to their new indices", () => {
    const moves = pageMoves(["a", "b", "c", "d"], ["c", "a", "b", "d"]);
    expect(moves.from).toEqual([2, 0, 1, 3]);
    expect([0, 1, 2, 3].map((old) => moves.to(old))).toEqual([1, 2, 0, 3]);
    expect(moves.added).toEqual([]);
  });

  it("sends a deleted page to the next page that stayed, else the one before", () => {
    const moves = pageMoves(["a", "b", "c", "d", "e"], ["a", "c", "d"]);
    expect(moves.to(1)).toBeUndefined();
    expect(moves.at(1)).toEqual({ page: 1, kept: false });
    expect(moves.at(4)).toEqual({ page: 2, kept: false });
    expect(moves.at(3)).toEqual({ page: 2, kept: true });
  });

  it("lists inserted pages as new", () => {
    const moves = pageMoves(["a", "b"], ["a", "x", "y", "b"]);
    expect(moves.from).toEqual([0, undefined, undefined, 1]);
    expect(moves.added).toEqual([1, 2]);
    expect(moves.at(1)).toEqual({ page: 3, kept: true });
  });

  it("reverses pages", () => {
    const moves = pageMoves(["a", "b", "c"], ["c", "b", "a"]);
    expect([0, 1, 2].map((old) => moves.at(old).page)).toEqual([2, 1, 0]);
  });
});
