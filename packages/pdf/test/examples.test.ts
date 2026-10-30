// The TS examples produce the Rust examples' bytes (fixtures/golden/examples/, ADR 0018 §5).
import { init } from "@lamina/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { generateThenEdit } from "../examples/generate-then-edit.js";
import { hello } from "../examples/hello.js";
import { imagesAndVectors } from "../examples/images-and-vectors.js";
import { invoice } from "../examples/invoice.js";
import { linksAndOutlines } from "../examples/links-and-outlines.js";
import { multilingual } from "../examples/multilingual.js";
import { read } from "./helpers.js";

const golden = (name: string): Uint8Array => read(`fixtures/golden/examples/${name}.pdf`);

beforeAll(() => init({ text: true }));

describe("examples", () => {
  it("hello builds the bytes of the Rust golden", () => {
    expect(hello()).toEqual(golden("hello"));
  });

  it("invoice builds the bytes of the Rust golden", () => {
    expect(invoice(read("fixtures/fonts/NotoSans-Regular.ttf"))).toEqual(golden("invoice"));
  });

  it("multilingual builds the bytes of the Rust golden", () => {
    const fonts = ["NotoSans-Regular.ttf", "NotoSansArabic-Regular.ttf", "NotoSansDevanagari-Regular.ttf"]
      .concat(["NotoSansHebrew-Regular.ttf", "NotoSansSC-Regular.otf"])
      .map((f) => read(`fixtures/fonts/${f}`));
    expect(multilingual(fonts)).toEqual(golden("multilingual"));
  });

  it("images-and-vectors builds the bytes of the Rust vectors golden", () => {
    expect(imagesAndVectors(read("fixtures/images/photo-rgb.jpg"))).toEqual(golden("vectors"));
  });

  it("links-and-outlines builds the bytes of the Rust golden", () => {
    expect(linksAndOutlines()).toEqual(golden("links_and_outlines"));
  });

  it("generate-then-edit builds the bytes of the Rust golden", () => {
    expect(generateThenEdit()).toEqual(golden("generate_then_edit"));
  });
});
