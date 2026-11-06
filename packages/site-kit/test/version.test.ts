// The install lines' version placeholder: every one replaced, and nothing else touched.

import { describe, expect, it } from "vitest";
import { VERSION_PLACEHOLDER, withVersion } from "../src/version.mjs";

describe("withVersion", () => {
  it("fills in every placeholder and leaves the rest of the text as it is", () => {
    const text = `npm install @laminapdf/sdk@${VERSION_PLACEHOLDER}\ncargo add laminapdf@=${VERSION_PLACEHOLDER}`;
    expect(withVersion(text, "0.2.0-alpha.3")).toBe(
      "npm install @laminapdf/sdk@0.2.0-alpha.3\ncargo add laminapdf@=0.2.0-alpha.3",
    );
    expect(withVersion("no placeholder {version}", "1.0.0")).toBe("no placeholder {version}");
  });
});
