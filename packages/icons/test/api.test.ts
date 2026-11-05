import { describe, expect, it } from "vitest";
import { ICONS, iconButtonProps, iconSvg } from "../src/index.ts";
import { Icon } from "../src/react/index.ts";

describe("iconSvg", () => {
  it("is decorative by default, at 16 px, stroked in currentColor", () => {
    const svg = iconSvg("find");
    expect(svg).toContain('width="16" height="16"');
    expect(svg).toContain('aria-hidden="true" focusable="false"');
    expect(svg).toContain('stroke="currentColor" stroke-width="1.5"');
    expect(svg).toContain(ICONS.find);
  });

  it("names an icon that stands alone, escaped", () => {
    const svg = iconSvg("warning", { size: 22, title: ' Repaired "file" ' });
    expect(svg).toContain('role="img" aria-label="Repaired &quot;file&quot;"');
    expect(svg).not.toContain("aria-hidden");
  });

  it("refuses an empty name", () => {
    expect(() => iconSvg("find", { title: " " })).toThrow(/non-empty accessible name/);
    expect(() => iconButtonProps("")).toThrow(/non-empty accessible name/);
    expect(iconButtonProps("Find in document")).toEqual({ "aria-label": "Find in document" });
  });

  it("types names and sizes", () => {
    const wrong = () => {
      // @ts-expect-error: not an icon
      iconSvg("nope");
      // @ts-expect-error: not a size the design language uses
      iconSvg("find", { size: 17 });
      // @ts-expect-error: not an icon
      Icon({ name: "nope" });
    };
    expect(typeof wrong).toBe("function");
  });
});
