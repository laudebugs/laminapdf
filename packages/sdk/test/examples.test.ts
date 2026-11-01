// The TS examples produce the Rust examples' bytes (fixtures/golden/examples/, ADR 0018 §5) in
// Node: the shared list (test/suite/examples.ts) every runtime of the B6c matrix runs. The
// report's hash and the performance smoke are test/node/measure.test.ts.
import { Builder, Font, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { openUntrusted } from "../examples/open-untrusted.js";
import { hello as nodeQuickStart } from "../examples/quick-start/node.js";
import { read } from "./helpers.js";
import { registerQuickStart, runSuite } from "./suite/examples.js";

beforeAll(() => init({ text: true }));

describe("examples", () => {
  runSuite("node", read, it, "examples");
  registerQuickStart("node", read, it, nodeQuickStart);

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

  it("opens an encrypted upload with its password and reports a missing or wrong one by code", () => {
    const file = read("fixtures/encrypted/r6-aes-256-user.pdf");
    expect(openUntrusted(file, "Pässwört-6")).toEqual({ ok: true, pages: 2, warnings: [] });
    expect(openUntrusted(file)).toMatchObject({ ok: false, code: "password-required" });
    expect(openUntrusted(file, "nope")).toMatchObject({ ok: false, code: "wrong-password" });
    expect(openUntrusted(new TextEncoder().encode("not a PDF"))).toMatchObject({ ok: false, code: "not-pdf" });
  });
});
