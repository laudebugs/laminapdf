import { describe, expect, it } from "vitest";
import pkg from "../package.json" with { type: "json" };
import { appVersion } from "../plugins/app-version.ts";

describe("the reader's version", () => {
  it("is package.json's when no release is named", () => {
    expect(appVersion({})).toBe(pkg.version);
  });

  it("is the release's when the deploy names one", () => {
    expect(appVersion({ LAMINA_PDF_WEB_VERSION: "0.1.2" })).toBe("0.1.2");
  });

  it("refuses a release that isn't MAJOR.MINOR.PATCH", () => {
    expect(() => appVersion({ LAMINA_PDF_WEB_VERSION: "v0.1" })).toThrow(/MAJOR\.MINOR\.PATCH/);
  });
});
