// The update test's pure parts (e2e/update/plan.ts). The test itself runs in CI on three OSes.
import { describe, expect, it } from "vitest";
import { emptyManifest, endpointPath, manifestFor, platformOf, requestProblems, tampered } from "../e2e/update/plan.ts";

const MAC = { target: "darwin", arch: "aarch64" };
const ID = "6f1c2b9e-3d4a-4f5b-9c8d-7e6f5a4b3c2d";
const VERSIONS = ["0.0.1", "0.0.2"];
const OLD = "/staging/darwin/aarch64/0.0.1";

describe("platformOf", () => {
  it("names an Apple silicon Mac as the Worker does", () => {
    expect(platformOf("darwin", "arm64")).toEqual(MAC);
  });

  it("names 64-bit Windows and Linux as the Worker does", () => {
    expect([platformOf("win32", "x64"), platformOf("linux", "x64")]).toEqual([
      { target: "windows", arch: "x86_64" },
      { target: "linux", arch: "x86_64" },
    ]);
  });

  it("refuses a platform the app isn't built for", () => {
    expect(() => platformOf("aix", "ppc64")).toThrow();
  });
});

describe("endpointPath", () => {
  it("asks the staging channel for the platform at the version", () => {
    expect(endpointPath(MAC, "0.0.1")).toBe(OLD);
  });
});

describe("manifestFor", () => {
  it("offers 0.0.2 to everyone on staging, with the signature file's text trimmed", () => {
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
        notes: "The desktop update test's release",
        rollout_percent: 100,
        platforms: { "darwin-aarch64": { url: "https://example.test/a.tar.gz", signature: "c2ln" } },
      },
    });
  });
});

describe("emptyManifest", () => {
  it("leaves the staging channel paused, offering nothing", () => {
    expect(emptyManifest()).toEqual({ schema: 1, channel: "staging", paused: true });
  });
});

describe("tampered", () => {
  it("changes exactly one byte", () => {
    const bytes = new Uint8Array([1, 2, 3, 4, 5]);
    expect([...tampered(bytes)].filter((byte, at) => byte !== bytes[at])).toHaveLength(1);
  });

  it("leaves the original bundle alone", () => {
    const bytes = new Uint8Array([1, 2, 3]);
    tampered(bytes);
    expect([...bytes]).toEqual([1, 2, 3]);
  });
});

describe("requestProblems", () => {
  it("accepts requests with the install ID only in the header", () => {
    const seen = [
      { url: OLD, installId: ID },
      { url: "/staging/darwin/aarch64/0.0.2", installId: ID },
    ];
    expect(requestProblems(seen, MAC, VERSIONS)).toEqual([]);
  });

  it("reports no request at all", () => {
    expect(requestProblems([], MAC, VERSIONS)).toEqual(["the app made no update request"]);
  });

  it("finds the install ID in the URL", () => {
    const seen = [
      { url: OLD, installId: ID },
      { url: `${OLD}?id=${ID}`, installId: ID },
    ];
    expect(requestProblems(seen, MAC, VERSIONS)).toContain(`${OLD}?id=${ID} holds the install ID`);
  });

  it("checks the updated version's requests too", () => {
    const seen = [
      { url: OLD, installId: ID },
      { url: `/staging/darwin/aarch64/0.0.2?id=${ID}`, installId: ID },
    ];
    expect(requestProblems(seen, MAC, VERSIONS)).toHaveLength(2);
  });

  it("reports a request without the header", () => {
    expect(requestProblems([{ url: OLD, installId: undefined }], MAC, VERSIONS)).toEqual([
      `${OLD} came without the lamina-install-id header`,
    ]);
  });

  it("reports an install ID that isn't a UUID v4", () => {
    expect(requestProblems([{ url: OLD, installId: "not-a-uuid" }], MAC, VERSIONS)).toEqual([
      `${OLD} sent an install ID that isn't a UUID v4`,
    ]);
  });

  it("wants a request from the version installed first", () => {
    expect(requestProblems([{ url: "/staging/darwin/aarch64/0.0.2", installId: ID }], MAC, VERSIONS)).toEqual([
      `no request asked for ${OLD}`,
    ]);
  });

  it("reports another channel's path", () => {
    const seen = [
      { url: OLD, installId: ID },
      { url: "/beta/darwin/aarch64/0.0.1", installId: ID },
    ];
    expect(requestProblems(seen, MAC, VERSIONS)).toHaveLength(1);
  });
});
