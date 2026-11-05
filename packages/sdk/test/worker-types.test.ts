// The worker client's types (L2a): every served member of the sync classes has a remote method of
// the same name returning a Promise of its result (a PageRef becomes a RemotePage).
import { describe, expectTypeOf, it } from "vitest";
import type { Document, PageRef } from "../src/document.js";
import type { Chunk, Extraction } from "../src/extract.js";
import type { PageText } from "../src/text.js";
import type { CallOptions, RemoteDocument, RemotePage } from "../src/worker/types.js";

describe("RemoteDocument", () => {
  it("mirrors Document's methods and getters as async methods", () => {
    expectTypeOf<RemoteDocument["pageCount"]>().toEqualTypeOf<(options?: CallOptions) => Promise<number>>();
    expectTypeOf<ReturnType<RemoteDocument["save"]>>().toEqualTypeOf<Promise<ReturnType<Document["save"]>>>();
    expectTypeOf<ReturnType<RemoteDocument["page"]>>().toEqualTypeOf<Promise<RemotePage>>();
    expectTypeOf<Parameters<RemoteDocument["pageInfo"]>[0]>().toEqualTypeOf<number>();
  });

  it("types extract by format, like the sync method", () => {
    const remote = {} as RemoteDocument;
    expectTypeOf(() => remote.extract({ format: "json" })).returns.toEqualTypeOf<Promise<Extraction>>();
    expectTypeOf(() => remote.extract({ format: "markdown" })).returns.toEqualTypeOf<Promise<string>>();
    expectTypeOf(() => remote.extract({ format: "chunks" }, { signal: AbortSignal.timeout(1) })).returns.toEqualTypeOf<
      Promise<Chunk[]>
    >();
  });

  it("has no members that take callbacks or step jobs", () => {
    expectTypeOf<RemoteDocument>().not.toHaveProperty("transact");
    expectTypeOf<RemoteDocument>().not.toHaveProperty("saveJob");
  });
});

describe("RemotePage", () => {
  it("mirrors PageRef", () => {
    expectTypeOf<ReturnType<RemotePage["text"]>>().toEqualTypeOf<Promise<PageText>>();
    expectTypeOf<ReturnType<RemotePage["index"]>>().toEqualTypeOf<Promise<PageRef["index"]>>();
  });
});
