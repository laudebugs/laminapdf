// WebKit's <select> report (src/webkit-select.ts): one console error and one violation per select
// come out in WebKit, and nothing more; other browsers keep everything.

import { REFUSED_STYLE, withoutSelectErrors, withoutSelectReports } from "@laminapdf/test-utils/webkit-select";
import { describe, expect, it } from "vitest";

const SELECT = { directive: "style-src-attr", target: "SELECT" };
const SPAN = { directive: "style-src-attr", target: "SPAN" };

describe("withoutSelectErrors", () => {
  it("takes out one refusal per select in WebKit, and keeps a third and any other error", () => {
    expect(withoutSelectErrors("webkit", [REFUSED_STYLE, "boom", REFUSED_STYLE, REFUSED_STYLE], 2)).toEqual([
      "boom",
      REFUSED_STYLE,
    ]);
  });

  it("keeps every refusal outside WebKit, or without a select", () => {
    expect([
      withoutSelectErrors("chromium", [REFUSED_STYLE], 2),
      withoutSelectErrors("webkit", [REFUSED_STYLE], 0),
    ]).toEqual([[REFUSED_STYLE], [REFUSED_STYLE]]);
  });
});

describe("withoutSelectReports", () => {
  it("takes out one style-src-attr report on a SELECT per select in WebKit, and keeps the rest", () => {
    expect(withoutSelectReports("webkit", [SELECT, SPAN, SELECT, SELECT], 2)).toEqual([SPAN, SELECT]);
  });

  it("keeps every report outside WebKit", () => {
    expect(withoutSelectReports("firefox", [SELECT], 1)).toEqual([SELECT]);
  });
});
