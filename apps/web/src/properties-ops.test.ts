// The Properties dialog's ops (`propertiesOps`, @laminapdf/ui) are shapes of the SDK's, which
// properties-editing.tsx hands on with a cast, since @laminapdf/ui doesn't depend on the SDK. The
// type check here holds them to the SDK's `Op`: every name one it takes, every argument one its op
// has, so an op or an argument renamed on either side fails here.

import type { Op } from "@laminapdf/sdk";
import { type PropertiesOp, propertiesOps } from "@laminapdf/ui/dialogs";
import { describe, expect, it } from "vitest";

/** `true` when every member of `A` is one of `B`'s. */
type Within<A, B> = Exclude<A, B> extends never ? true : false;

/** The arguments of op `N` in the union `O`. */
type ArgsOf<O, N> = O extends { readonly op: N; readonly args: infer A } ? A : never;

/** For each of the dialog's ops: whether the SDK has it, and its argument names are all the SDK op's. */
type ArgumentsWithin = {
  [N in PropertiesOp["op"]]: [ArgsOf<Op, N>] extends [never]
    ? false
    : Within<keyof ArgsOf<PropertiesOp, N>, keyof ArgsOf<Op, N>>;
};

const NAMES: Within<PropertiesOp["op"], Op["op"]> = true;
const ARGUMENTS: ArgumentsWithin = {
  setMetadata: true,
  removeMetadata: true,
  setXmpProperty: true,
  removeXmpProperty: true,
  replaceXmpPacket: true,
};

describe("the Properties dialog's ops", () => {
  it("are ops the SDK takes, with its argument names", () => {
    expect([NAMES, Object.values(ARGUMENTS).every(Boolean)]).toEqual([true, true]);
  });

  it("put the form's change after the XMP tab's", () => {
    const ops = propertiesOps({ patch: { title: "Report" }, remove: [] }, [{ kind: "replace", xml: "<x:xmpmeta/>" }], {
      replaceMalformed: false,
    });
    expect(ops.map((op) => op.op)).toEqual(["replaceXmpPacket", "setMetadata"]);
  });
});
