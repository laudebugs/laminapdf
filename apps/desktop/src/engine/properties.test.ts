// The Properties dialog's data as the engine sends it (src/engine/properties.fixture.json, which
// laminapdf-desktop-engine's `properties` test writes from the engine's own answer): every key
// it holds is one @laminapdf/ui's shapes read, so a field renamed on either side fails the type
// check here.

import { describe, expect, it } from "vitest";
import type { DesktopProperties } from "./desktop-engine-client.ts";
import fixture from "./properties.fixture.json" with { type: "json" };

/** `true` when every key of `A` is one of `B`'s. */
type KeysWithin<A, B> = Exclude<keyof A, keyof B> extends never ? true : false;

const PROPERTIES_KEYS: KeysWithin<typeof fixture.properties, DesktopProperties["properties"]> = true;
// The metadata is the SDK's `DocumentMetadata`, which also says which side won and which Info keys a
// PDF 2.0 file keeps: the dialog shows neither.
const METADATA_KEYS: KeysWithin<
  Omit<typeof fixture.metadata, "source" | "keptInfoKeys">,
  DesktopProperties["metadata"]
> = true;
const XMP_KEYS: KeysWithin<typeof fixture.xmp, DesktopProperties["xmp"]> = true;
const FONT_KEYS: KeysWithin<
  (typeof fixture.properties.fonts)[number],
  DesktopProperties["properties"]["fonts"][number]
> = true;

describe("the engine's Properties data", () => {
  it("holds only keys the dialog reads", () => {
    expect([PROPERTIES_KEYS, METADATA_KEYS, XMP_KEYS, FONT_KEYS]).toEqual([true, true, true, true]);
  });

  it("carries the facts, the metadata and the packet the dialog shows", () => {
    expect([fixture.properties.pageCount, fixture.metadata.title, fixture.xmp.status]).toEqual([
      1,
      "Metadata golden: Info & XMP <in sync>",
      "parsed",
    ]);
  });
});
