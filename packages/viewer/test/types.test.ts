// The viewer's RPC types (C3a): `call` takes only known methods, with their params, and returns
// their results.
import type { PageInfo } from "@lamina/pdf";
import { describe, expectTypeOf, it } from "vitest";
import type { ViewerClient } from "../src/client.js";
import type { OpenedDocument, RenderedBitmap } from "../src/protocol.js";

describe("ViewerClient.call", () => {
  it("is typed by the method map", () => {
    type Call = ViewerClient["call"];
    expectTypeOf<ReturnType<ViewerClient["render"]>>().toEqualTypeOf<Promise<RenderedBitmap>>();
    expectTypeOf<ReturnType<ViewerClient["open"]>>().toEqualTypeOf<Promise<OpenedDocument>>();
    expectTypeOf<ReturnType<ViewerClient["pageInfo"]>>().toEqualTypeOf<Promise<PageInfo>>();
    expectTypeOf<Call>().toBeCallableWith("pageInfo", [0]);
  });

  it("rejects unknown methods and wrong params", () => {
    const call = null as unknown as ViewerClient["call"];
    const never = () => {
      // @ts-expect-error: no such method
      void call("nope", []);
      // @ts-expect-error: pageInfo takes a number
      void call("pageInfo", ["0"]);
      // @ts-expect-error: render needs a request
      void call("render", []);
    };
    void never;
  });
});
