import { describe, expect, it } from "vitest";
import { type CustomStamp, imageType, StampLibrary, type StampStore, stampName } from "./stamp-library.ts";

/** A store over a map, as IndexedDB keeps the stamps by ID. */
function mapStore(): StampStore {
  const kept = new Map<string, CustomStamp>();
  return {
    all: async () => [...kept.values()],
    put: async (stamp) => {
      kept.set(stamp.id, stamp);
    },
    delete: async (id) => {
      kept.delete(id);
    },
  };
}

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0]);
const JPEG = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0, 0]);

/** A library whose clock and IDs count up, so its order is known. */
function library(): StampLibrary {
  let clock = 0;
  let next = 0;
  return new StampLibrary(
    mapStore(),
    () => ++clock,
    () => `s${++next}`,
  );
}

describe("imageType", () => {
  it("knows PNG and JPEG by their bytes, whatever the file is called", () => {
    expect([imageType(PNG), imageType(JPEG), imageType(new TextEncoder().encode("GIF89a"))]).toEqual([
      "image/png",
      "image/jpeg",
      undefined,
    ]);
  });
});

describe("stampName", () => {
  it("drops the extension, and keeps a name that is only an extension", () => {
    expect([stampName("Company seal.png"), stampName(".png")]).toEqual(["Company seal", ".png"]);
  });
});

describe("StampLibrary", () => {
  it("lists the stamps newest first", async () => {
    const stamps = library();
    await stamps.add("seal.png", PNG);
    await stamps.add("logo.jpg", JPEG);
    expect((await stamps.list()).map((s) => [s.name, s.type])).toEqual([
      ["logo", "image/jpeg"],
      ["seal", "image/png"],
    ]);
  });

  it("keeps nothing for a file that isn't a PNG or JPEG", async () => {
    const stamps = library();
    const added = await stamps.add("notes.png", new TextEncoder().encode("not an image"));
    expect([added, await stamps.list()]).toEqual([undefined, []]);
  });

  it("renames a stamp, keeping it for a blank name, and removes one", async () => {
    const stamps = library();
    const seal = await stamps.add("seal.png", PNG);
    const logo = await stamps.add("logo.png", PNG);
    await stamps.rename(seal?.id ?? "", "  Company seal ");
    await stamps.rename(logo?.id ?? "", "   ");
    await stamps.remove(logo?.id ?? "");
    expect((await stamps.list()).map((s) => s.name)).toEqual(["Company seal"]);
  });
});
