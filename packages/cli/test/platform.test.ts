import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  binaryName,
  libcFromReport,
  missingPackageMessage,
  type Platform,
  platformFor,
  unsupportedMessage,
} from "../src/platform.js";

const PLATFORMS = JSON.parse(readFileSync(new URL("../platforms.json", import.meta.url), "utf8")) as Platform[];

describe("platformFor", () => {
  it.each([
    [{ os: "darwin", cpu: "arm64" }, "@laminapdf/cli-darwin-arm64"],
    [{ os: "darwin", cpu: "x64" }, "@laminapdf/cli-darwin-x64"],
    [{ os: "linux", cpu: "x64", libc: "glibc" }, "@laminapdf/cli-linux-x64-gnu"],
    [{ os: "linux", cpu: "arm64", libc: "glibc" }, "@laminapdf/cli-linux-arm64-gnu"],
    [{ os: "linux", cpu: "x64", libc: "musl" }, "@laminapdf/cli-linux-x64-musl"],
    [{ os: "linux", cpu: "arm64", libc: "musl" }, "@laminapdf/cli-linux-arm64-musl"],
    [{ os: "win32", cpu: "x64" }, "@laminapdf/cli-win32-x64"],
    [{ os: "win32", cpu: "arm64" }, "@laminapdf/cli-win32-arm64"],
  ] as const)("picks the package for %o", (host, name) => {
    expect(platformFor(PLATFORMS, host)?.package).toBe(name);
  });

  it.each([
    { os: "freebsd", cpu: "x64" },
    { os: "linux", cpu: "ia32", libc: "glibc" },
    { os: "win32", cpu: "ia32" },
  ] as const)("finds none for %o", (host) => {
    expect(platformFor(PLATFORMS, host)).toBeUndefined();
  });
});

describe("libcFromReport", () => {
  it("reads glibc from a report that names its version", () => {
    expect(libcFromReport({ header: { glibcVersionRuntime: "2.39" } })).toBe("glibc");
  });

  it("reads musl from a report without one", () => {
    expect(libcFromReport({ header: { osName: "Linux" } })).toBe("musl");
  });
});

describe("binaryName", () => {
  it("adds .exe on Windows only", () => {
    const names = PLATFORMS.map((p) => [p.os, binaryName(p)]);
    expect(new Set(names.map(([os, name]) => `${os}:${name}`))).toEqual(
      new Set(["darwin:laminapdf", "linux:laminapdf", "win32:laminapdf.exe"]),
    );
  });
});

describe("unsupportedMessage", () => {
  it("names the host and every supported platform", () => {
    const message = unsupportedMessage(PLATFORMS, { os: "freebsd", cpu: "x64" });
    expect(message).toContain("no prebuilt binary for freebsd x64");
    for (const line of ["darwin arm64", "linux arm64 (musl)", "linux x64 (glibc)", "win32 x64"]) {
      expect(message).toContain(`  ${line}\n`);
    }
  });
});

describe("missingPackageMessage", () => {
  it("names the package and the install flag that leaves it out", () => {
    const linux = platformFor(PLATFORMS, { os: "linux", cpu: "x64", libc: "glibc" });
    expect(linux).toBeDefined();
    if (linux === undefined) return;
    const message = missingPackageMessage(linux);
    expect(message).toContain("@laminapdf/cli-linux-x64-gnu");
    expect(message).toContain("--omit=optional");
  });
});
