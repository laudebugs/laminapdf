// The build steps' HTML reader (src/markup.mjs): it splits a page as a browser does, so text that
// looks like a tag inside an attribute, a script or a comment is never read as one.

import { describe, expect, it } from "vitest";
import { decodeReferences, startTags, tokens } from "../src/markup.mjs";

describe("tokens", () => {
  it("gives the page back when its pieces are joined", () => {
    const page = `<!doctype html><p class=a>x &amp; y</p><!-- c --><script src="/a.js">if (a<b) {}</script><br/>z < w`;
    expect(
      tokens(page)
        .map((token) => token.text)
        .join(""),
    ).toBe(page);
  });
});

describe("startTags", () => {
  it("reads attributes in every quoting, and keeps a value holding markup whole", () => {
    expect(startTags(`<button data-code="a <span style='c'> b" class=copy disabled>`)).toEqual([
      {
        name: "button",
        attributes: new Map([
          ["data-code", "a <span style='c'> b"],
          ["class", "copy"],
          ["disabled", ""],
        ]),
      },
    ]);
  });

  it("reads a script's and a style's start tag, not their bodies", () => {
    expect(startTags(`<script src="/a.js">"<i style=z>"</script><style media=print>p{}</style>`)).toEqual([
      { name: "script", attributes: new Map([["src", "/a.js"]]) },
      { name: "style", attributes: new Map([["media", "print"]]) },
    ]);
  });
});

describe("decodeReferences", () => {
  it("decodes named, decimal and hexadecimal references, and leaves unknown ones", () => {
    expect(decodeReferences("&quot;a&quot; &#39;b&#x27; &amp; &unknown;")).toBe(`"a" 'b' & &unknown;`);
  });
});
