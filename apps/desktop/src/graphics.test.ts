// Settings › Graphics drawn to HTML in Node: its choices, Restart now after a change, the
// crash-loop guard's notice and a `--safe-graphics` start.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { type Graphics, GraphicsSettings, type GraphicsStatus } from "./graphics.tsx";

const RUNNING_AUTOMATIC: GraphicsStatus = { applies: true, running: "automatic", set: [], fellBack: false };

/** The section's text for `status` with `saved` chosen, its tags dropped. */
function text(status: Partial<GraphicsStatus>, saved: Graphics = "automatic"): string {
  const controls = {
    status: { ...RUNNING_AUTOMATIC, ...status },
    saved,
    choose: vi.fn(),
    restart: vi.fn(async () => true),
  };
  return renderToStaticMarkup(createElement(GraphicsSettings, { controls }))
    .replace(/<[^>]+>/g, " ")
    .replaceAll("&#x27;", "'");
}

describe("GraphicsSettings", () => {
  it("offers Automatic, Compatibility and Safe mode", () => {
    expect(["Automatic", "Compatibility", "Safe mode"].every((name) => text({}).includes(name))).toBe(true);
  });

  it("offers Restart now when the saved choice isn't the one running", () => {
    expect(text({}, "compatibility")).toContain("Restart now");
  });

  it("offers no restart when the running choice is the saved one", () => {
    expect(text({}, "automatic")).not.toContain("Restart now");
  });

  it("says when the guard switched to Compatibility", () => {
    expect(text({ running: "compatibility", fellBack: true }, "compatibility")).toContain(
      "Lamina PDF switched to Compatibility.",
    );
  });

  it("says Safe mode lasts until quitting after --safe-graphics, with no restart", () => {
    const shown = text({ running: "safe" }, "automatic");
    expect([shown.includes("--safe-graphics"), shown.includes("Restart now")]).toEqual([true, false]);
  });
});
