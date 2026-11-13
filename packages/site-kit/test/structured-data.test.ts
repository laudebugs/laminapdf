// The JSON-LD both home pages carry: built from the workspace's Cargo.toml, shaped as schema.org's
// types need, with no rating, review or offer, and safe inside a script element.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { jsonLd, projectFacts, structuredData, structuredDataProblems } from "../src/structured-data.mjs";

const CARGO = readFileSync(new URL("../../../Cargo.toml", import.meta.url), "utf8");

describe("projectFacts", () => {
  it("reads the licence, the repository and the home page from Cargo.toml", () => {
    expect(projectFacts(CARGO)).toEqual({
      licence: "MIT OR Apache-2.0",
      licences: ["https://spdx.org/licenses/MIT.html", "https://spdx.org/licenses/Apache-2.0.html"],
      repository: "https://github.com/laudebugs/laminapdf",
      homepage: "https://laminapdf.com",
    });
  });

  it("fails when a fact is missing, rather than describe the project wrongly", () => {
    expect(() => projectFacts('[workspace.package]\nlicense = "MIT"\n')).toThrow("has no repository");
  });
});

describe("structuredData", () => {
  for (const site of ["website", "docs"] as const) {
    it(`describes the ${site} with no problems`, () => {
      expect(structuredDataProblems(structuredData({ site, cargoToml: CARGO }))).toEqual([]);
    });
  }

  it("names each site's own address, and the same publisher and code on both", () => {
    const [website, docs] = (["website", "docs"] as const).map(
      (site) => structuredData({ site, cargoToml: CARGO })["@graph"] as Record<string, unknown>[],
    );
    expect(website?.[1]?.url).toBe("https://laminapdf.com/");
    expect(docs?.[1]?.url).toBe("https://docs.laminapdf.com/");
    expect(website?.[0]).toEqual(docs?.[0]);
    expect(website?.[2]).toEqual(docs?.[2]);
  });
});

describe("structuredDataProblems", () => {
  const good = () => structuredData({ site: "website", cargoToml: CARGO });

  it("refuses a rating, a review or an offer anywhere", () => {
    const data = good();
    (data["@graph"] as Record<string, unknown>[])[2] = {
      ...(data["@graph"] as Record<string, unknown>[])[2],
      aggregateRating: { ratingValue: 5 },
      offers: { price: 0 },
    };
    expect(structuredDataProblems(data)).toEqual([
      "carries aggregateRating, which Lamina PDF doesn't have",
      "carries offers, which Lamina PDF doesn't have",
    ]);
  });

  it("names a missing property, an unknown type, a plain http address and a dangling reference", () => {
    const graph = good()["@graph"] as Record<string, unknown>[];
    const [organization, website, code] = graph;
    const data = {
      "@context": "https://schema.org",
      "@graph": [
        { ...organization, logo: undefined, url: "http://laminapdf.com/" },
        { ...website, publisher: { "@id": "https://example.com/#nobody" } },
        code,
        { "@type": "Product", "@id": "https://laminapdf.com/#product" },
      ],
    };
    expect(structuredDataProblems(data)).toEqual([
      "Organization has no logo",
      "unexpected type Product",
      "url is http://laminapdf.com/, not an https: address",
      "refers to https://example.com/#nobody, which the graph doesn't define",
    ]);
  });
});

describe("jsonLd", () => {
  it("escapes < so no string can close the script element", () => {
    const text = jsonLd({ name: "</script><script>alert(1)</script>" });
    expect(text).not.toContain("<");
    expect(JSON.parse(text)).toEqual({ name: "</script><script>alert(1)</script>" });
  });
});
