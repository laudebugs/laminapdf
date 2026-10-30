// Builder, Page and Surface in the base module: drawing, errors, and freeing in a loop.
import { Builder, Document, Image, init, isLaminaError, Matrix, wasmMemory } from "@lamina/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { thrown } from "./helpers.js";

/** A one-page document drawn with `draw`, saved and reopened. */
function drawn(draw: (b: Builder, page: ReturnType<Builder["page"]>) => void): Document {
  const b = new Builder({ id: "000102030405060708090a0b0c0d0e0f", creationDate: "D:20261030120000Z" });
  const page = b.page("A5");
  draw(b, page);
  page.finish();
  return Document.open(b.finish().save());
}

beforeAll(() => init());

describe("Builder", () => {
  it("draws paths, colours, images and links into a valid file", () => {
    const doc = drawn((_, page) => {
      page.cropBox([10, 10, 400, 500]).rotate(90);
      page
        .surface()
        .fillColor([0.1, 0.2, 0.3, 0.4])
        .path((p) => p.moveTo(20, 20).lineTo(100, 20).quadTo(120, 60, 100, 100).close())
        .fillStroke("evenOdd", { width: 2, join: "round" })
        .transform(Matrix.scale(2, 2))
        .image(Image.png(pngPixel()), [10, 10, 20, 20])
        .link([10, 10, 50, 20], { type: "uri", uri: "https://example.com/" });
    });
    expect([doc.pageCount, doc.pageInfo(0).rotation, Math.round(doc.pageInfo(0).width)]).toEqual([1, 90, 400]);
  });

  it("gives a page size by name or in points", () => {
    const b = new Builder();
    b.page("Letter").finish();
    b.page([100, 200]).finish();
    expect(b.finish().pages.map((p) => [p.width, p.height])).toEqual([
      [612, 792],
      [100, 200],
    ]);
  });

  it("reports invalid geometry when the page finishes", () => {
    const b = new Builder();
    const page = b.page("A6");
    page
      .surface()
      .path((p) => p.rect([0, 0, Number.NaN, 1]))
      .fill();
    expect(
      isLaminaError(
        thrown(() => page.finish()),
        "invalid-geometry",
      ),
    ).toBe(true);
  });

  it("reports a bad user unit as a LaminaError", () => {
    const page = new Builder().page("A6");
    const e = thrown(() => page.userUnit(Number.NaN).finish());
    expect(isLaminaError(e)).toBe(true);
  });

  it("refuses an unknown blend mode with suggestions", () => {
    const surface = new Builder().page("A6").surface();
    const e = thrown(() => surface.blendMode("multiplly" as never));
    expect(isLaminaError(e, "invalid-argument") && e.didYouMean).toEqual(["multiply"]);
  });

  it("refuses drawing on a finished page (builder-finished)", () => {
    const page = new Builder().page("A6");
    page.finish();
    expect(
      isLaminaError(
        thrown(() => page.surface().save()),
        "builder-finished",
      ),
    ).toBe(true);
  });

  it("reports an RGBA image of the wrong length when it is first decoded", () => {
    const image = Image.rgba(2, 2, new Uint8Array(3));
    expect(
      isLaminaError(
        thrown(() => image.width),
        "image-data-length",
      ),
    ).toBe(true);
  });

  it("copies an image's bytes, so changing the input later changes nothing", () => {
    const png = pngPixel();
    const image = Image.png(png);
    png.fill(0);
    expect(drawn((_, page) => page.surface().image(image, [0, 0, 10, 10])).pageCount).toBe(1);
  });

  it("refuses an unknown page size name with suggestions", () => {
    const e = thrown(() => new Builder().page("A44" as never));
    expect(isLaminaError(e, "invalid-argument") && e.didYouMean?.[0]).toBe("A4");
  });

  it("frees unfinished pages with the builder (builder-finished afterwards)", () => {
    const b = new Builder();
    const page = b.page("A6");
    b.free();
    expect(
      isLaminaError(
        thrown(() => page.finish()),
        "builder-finished",
      ),
    ).toBe(true);
  });

  it("frees 1,000 builders and documents without growing memory", () => {
    const build = () => {
      const b = new Builder();
      const page = b.page("A6");
      page
        .surface()
        .path((p) => p.circle([50, 50], 20))
        .fill();
      page.finish();
      const doc = b.finish();
      doc.save();
      doc.free();
      b.free();
    };
    for (let i = 0; i < 50; i++) build();
    const before = wasmMemory().base ?? 0;
    for (let i = 0; i < 1000; i++) build();
    expect(wasmMemory().base).toBe(before);
  });
});

/** A 1×1 red PNG. */
function pngPixel(): Uint8Array {
  const b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}
