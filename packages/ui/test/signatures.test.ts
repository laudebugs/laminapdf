import { describe, expect, it } from "vitest";
import { openDatabase } from "../src/platform/idb.ts";
import { createSignatureLibrary, SIGNATURES_LIMIT } from "../src/platform/signatures.ts";
import type { SavedSignature } from "../src/platform/types.ts";
import { createWebPlatform } from "../src/platform/web.ts";
import { fakeScope } from "./fakes.ts";

function signature(id: string, added: number, name = id): SavedSignature {
  return {
    id,
    name,
    kind: "drawn",
    bytes: new Uint8Array([0x89, 0x50]),
    type: "image/png",
    width: 300,
    height: 120,
    added,
  };
}

describe("the signature library", () => {
  it("lists signatures newest first", async () => {
    const library = createSignatureLibrary(openDatabase(undefined));
    await library.put(signature("a", 1));
    await library.put(signature("b", 3));
    await library.put(signature("c", 2));
    expect((await library.list()).map((s) => s.id)).toEqual(["b", "c", "a"]);
  });

  it("replaces a signature with the same ID instead of adding a second", async () => {
    const library = createSignatureLibrary(openDatabase(undefined));
    await library.put(signature("a", 1, "First"));
    await library.put(signature("a", 1, "Renamed"));
    expect(await library.list()).toHaveLength(1);
    expect((await library.list())[0]?.name).toBe("Renamed");
  });

  it("removes one signature, then all", async () => {
    const library = createSignatureLibrary(openDatabase(undefined));
    await library.put(signature("a", 1));
    await library.put(signature("b", 2));
    await library.remove("a");
    expect((await library.list()).map((s) => s.id)).toEqual(["b"]);
    await library.clear();
    expect(await library.list()).toEqual([]);
  });

  it("keeps only the newest signatures past its limit", async () => {
    const library = createSignatureLibrary(openDatabase(undefined), 3);
    for (let i = 1; i <= 5; i += 1) await library.put(signature(`s${i}`, i));
    expect((await library.list()).map((s) => s.id)).toEqual(["s5", "s4", "s3"]);
    expect(SIGNATURES_LIMIT).toBeGreaterThan(3);
  });

  it("is part of the web platform, in memory where IndexedDB doesn't open", async () => {
    const platform = createWebPlatform(fakeScope());
    expect(platform.capabilities.signatureLibrary).toBe(true);
    await platform.signatures.put(signature("a", 1));
    expect((await platform.signatures.list())[0]?.id).toBe("a");
  });

  it("is not emptied by clearing the platform's storage", async () => {
    const platform = createWebPlatform(fakeScope());
    await platform.signatures.put(signature("a", 1));
    await platform.storage.clear();
    expect(await platform.signatures.list()).toHaveLength(1);
  });
});
