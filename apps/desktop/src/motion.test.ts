// Reduced motion in the desktop window (motion.ts): Settings' choice, or the system's setting
// where the main process reads it.
import { describe, expect, it } from "vitest";
import { reducesMotion } from "./motion.ts";

describe("reducesMotion", () => {
  it("follows the system's setting while Settings says Match system", () => {
    expect([reducesMotion("system", true), reducesMotion("system", false), reducesMotion("system", null)]).toEqual([
      true,
      false,
      false,
    ]);
  });

  it("reduces motion whatever the system says when Settings says so", () => {
    expect([reducesMotion("reduce", true), reducesMotion("reduce", false), reducesMotion("reduce", null)]).toEqual([
      true,
      true,
      true,
    ]);
  });
});
