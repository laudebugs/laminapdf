// The TS examples produce the Rust examples' bytes (fixtures/golden/examples/, ADR 0018 §5).
import { Builder, Font, init } from "@lamina/pdf";
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

  it("uses one PDF font for equal fonts created per call", () => {
    const build = (font: () => Font) => {
      const b = new Builder({ id: "000102030405060708090a0b0c0d0e0f", creationDate: "D:20261030120000Z" });
      const page = b.page("A6");
      page.surface().text("one", { font: font(), size: 12, at: [10, 20] });
      page.surface().text("two", { font: font(), size: 12, at: [10, 40] });
      const noto = Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf"));
      page.surface().text("three", { font: noto, size: 12, at: [10, 60] });
      page
        .surface()
        .text("four", { font: Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf")), size: 12, at: [10, 80] });
      page.finish();
      return b.finish().save();
    };
    const helvetica = Font.standard("Helvetica");
    expect(build(() => Font.standard("Helvetica"))).toEqual(build(() => helvetica));
  });

  it("generate-then-edit builds the bytes of the Rust golden", () => {
    expect(generateThenEdit()).toEqual(golden("generate_then_edit"));
  });
});
