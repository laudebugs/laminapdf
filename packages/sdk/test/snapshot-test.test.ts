// The snapshot example: its four answers, and that its options give the Rust example's bytes.
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { init } from "@laminapdf/sdk";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { hello } from "../examples/hello.js";
import { compareWithReference, TEST_OPTIONS } from "../examples/snapshot-test.js";

/** `hello` built with `TEST_OPTIONS`; `crates/laminapdf/tests/examples.rs` reads it too. */
export const SNAPSHOT_HELLO_SHA256 = "84b43584aedb408a811c34b9777deda9b9cd6130fd5804d4d9c87ecc4461f1f4";

const dir = mkdtempSync(join(tmpdir(), "laminapdf-snapshot-"));
const reference = join(dir, "hello.pdf");
const pending = join(dir, "hello.new.pdf");

beforeAll(() => init({ text: true }));
afterAll(() => rmSync(dir, { recursive: true, force: true }));

describe("snapshot-test example", () => {
  it("gives the bytes the Rust example's test_options() give", () => {
    expect(createHash("sha256").update(hello(TEST_OPTIONS)).digest("hex")).toBe(SNAPSHOT_HELLO_SHA256);
  });

  it("creates the reference, matches it, reports a change beside it and updates it", () => {
    const approved = hello(TEST_OPTIONS);
    const other = hello({ ...TEST_OPTIONS, producer: "another app" });
    expect(compareWithReference(approved, reference)).toBe("created");
    expect(compareWithReference(approved, reference)).toBe("same");
    expect(compareWithReference(other, reference)).toBe("changed");
    expect(new Uint8Array(readFileSync(reference))).toEqual(approved);
    expect(new Uint8Array(readFileSync(pending))).toEqual(other);
    expect(compareWithReference(other, reference, { update: true })).toBe("changed");
    expect(new Uint8Array(readFileSync(reference))).toEqual(other);
    expect(existsSync(pending)).toBe(false);
  });
});
