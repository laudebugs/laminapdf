import { describe, expect, it } from "vitest";
import { emptyManifest, manifestFor, platformOf, requestProblems, tampered } from "../e2e/update/plan.ts";

const MAC = { target: "darwin", arch: "aarch64" };
const ID = "6f1c2b9e-3d4a-4f5b-9c8d-7e6f5a4b3c2d";

describe("the update test's plan", () => {
  it("names the platforms the Worker serves", () => {
    expect(platformOf("darwin", "arm64")).toEqual(MAC);
    expect(platformOf("win32", "x64")).toEqual({ target: "windows", arch: "x86_64" });
    expect(platformOf("linux", "x64")).toEqual({ target: "linux", arch: "x86_64" });
    expect(() => platformOf("aix", "ppc64")).toThrow();
  });

  it("offers 0.0.2 on staging, with the signature file's text trimmed", () => {
    const manifest = manifestFor(
      "darwin-aarch64",
      "https://example.test/a.tar.gz",
      "c2ln\n",
      new Date(Date.UTC(2026, 10, 15, 12)),
    );
    expect(manifest).toEqual({
      schema: 1,
      channel: "staging",
      paused: false,
      release: {
        version: "0.0.2",
        pub_date: "2026-11-15T12:00:00Z",
        notes: "The desktop update test's release (D5c)",
        rollout_percent: 100,
        platforms: { "darwin-aarch64": { url: "https://example.test/a.tar.gz", signature: "c2ln" } },
      },
    });
    expect(emptyManifest()).toEqual({ schema: 1, channel: "staging", paused: true });
  });

  it("changes exactly one byte of a tampered bundle", () => {
    const bytes = new Uint8Array([1, 2, 3, 4, 5]);
    const changed = tampered(bytes);
    expect([...changed].filter((byte, at) => byte !== bytes[at])).toHaveLength(1);
    expect([...bytes]).toEqual([1, 2, 3, 4, 5]);
  });

  it("accepts requests with the install ID only in the header", () => {
    expect(requestProblems([{ url: "/staging/darwin/aarch64/0.0.1", installId: ID }], MAC, "0.0.1")).toEqual([]);
  });

  it("finds the install ID in the URL, a missing header and a wrong path", () => {
    expect(requestProblems([], MAC, "0.0.1")).toEqual(["the app made no update request"]);
    expect(
      requestProblems([{ url: `/staging/darwin/aarch64/0.0.1?id=${ID}`, installId: ID }], MAC, "0.0.1"),
    ).toHaveLength(2);
    expect(
      requestProblems([{ url: "/staging/darwin/aarch64/0.0.1", installId: undefined }], MAC, "0.0.1"),
    ).toHaveLength(1);
    expect(
      requestProblems([{ url: "/beta/darwin/aarch64/0.0.1", installId: "not-a-uuid" }], MAC, "0.0.1"),
    ).toHaveLength(2);
  });
});
