import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { EXAMPLES } from "../states.tsx";
import { cleanup, renderIn, type Setting } from "./render.tsx";

const SETTINGS: Setting[] = (["light", "dark", "high-contrast"] as const).flatMap((theme) =>
  (["ltr", "rtl"] as const).map((dir) => ({ theme, dir })),
);

afterEach(cleanup);

describe.each(SETTINGS)("axe, $theme theme, $dir", (setting) => {
  it.each(EXAMPLES.map((e) => [`${e.component}: ${e.state}`, e] as const))("%s", async (_name, example) => {
    const host = renderIn(example.element(), setting);
    const result = await axe.run(host, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
    });
    expect(result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  });
});
