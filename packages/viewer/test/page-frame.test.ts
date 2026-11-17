// @laminapdf/viewer's pageToUser against fixtures/geometry/page-to-user.json, the cases
// laminapdf-model's PageFrame::to_user_space reads too (tests/page_frame.rs), so the reader's
// annotation tools map page space to user space as the model does.
// biome-ignore-all lint/correctness/noNodejsModules: this Node-only test reads the shared fixture from the repository.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { pageToUser } from "../src/layers/annotation-geometry.js";

describe("pageToUser", () => {
  it("maps page space to user space as the model does, on the shared cases", () => {
    const fixture = JSON.parse(
      readFileSync(new URL("../../../fixtures/geometry/page-to-user.json", import.meta.url), "utf8"),
    ) as {
      cases: {
        crop: [number, number, number, number];
        rotate: number;
        userUnit: number;
        page: [number, number];
        user: [number, number];
      }[];
    };
    for (const { crop, rotate, userUnit, page, user } of fixture.cases) {
      const got = pageToUser({ crop, rotate, unit: userUnit }, page);
      expect([got.x, got.y].map((v) => Math.round(v * 1e9) / 1e9)).toEqual(user.map((v) => Math.round(v * 1e9) / 1e9));
    }
  });
});
