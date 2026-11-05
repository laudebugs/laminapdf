import { describe, expect, it } from "vitest";
import { galleryHtml } from "../gallery/gallery.ts";
import { ICONS } from "../src/index.ts";

describe("the gallery", () => {
  it("shows every icon in both themes", () => {
    const html = galleryHtml();
    for (const name of Object.keys(ICONS)) {
      expect(html.match(new RegExp(`data-icon="${name}"`, "g")), name).toHaveLength(2);
    }
  });
});
