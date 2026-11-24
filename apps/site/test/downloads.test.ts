// The download page's data (src/data/downloads.ts): which lists it accepts, the system it guesses
// from a browser, what it offers first, and the table's order. The e2e suite checks the same
// choices in each browser (e2e/download.e2e.test.ts).
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  cardFiles,
  type DownloadFile,
  type Downloads,
  offered,
  offeredPlatforms,
  parseDownloads,
  parseSnapshot,
  tableOrder,
  visitorFrom,
} from "../src/data/downloads.ts";

const FIXTURE = readFileSync(new URL("../e2e/downloads.fixture.json", import.meta.url), "utf8");
const BETA = (JSON.parse(FIXTURE) as { beta: { files: DownloadFile[] } }).beta;
const MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_6) AppleWebKit/605.1.15 Version/18.0 Safari/605.1.15";
const WINDOWS_ARM = "Mozilla/5.0 (Windows NT 10.0; ARM64) AppleWebKit/537.36 Chrome/141.0 Safari/537.36";
const CHROMEBOOK = "Mozilla/5.0 (X11; CrOS x86_64 15000.0.0) AppleWebKit/537.36 Chrome/141.0 Safari/537.36";

describe("parseSnapshot", () => {
  it("reads the e2e fixture's beta and nightly, and no stable", () => {
    const snapshot = parseSnapshot(FIXTURE);
    expect([snapshot.beta?.version, snapshot.stable, snapshot.nightly?.files.length]).toEqual(["0.1.0", null, 8]);
  });

  it("reads the committed snapshot", () => {
    expect(() =>
      parseSnapshot(readFileSync(new URL("../src/data/downloads.json", import.meta.url), "utf8")),
    ).not.toThrow();
  });
});

describe("parseDownloads", () => {
  it("refuses a file served from another host", () => {
    const list = { ...JSON.parse(FIXTURE).beta, files: [{ ...BETA.files[0], url: "https://example.com/a.dmg" }] };
    expect(parseDownloads(list, "beta")).toBe("file 0 is malformed");
  });

  it("refuses another channel's list", () => {
    expect(parseDownloads(JSON.parse(FIXTURE).beta, "stable")).toBe("channel beta, not stable");
  });
});

describe("visitorFrom", () => {
  it("leaves a Mac's processor unknown without client hints", () => {
    expect(visitorFrom(undefined, MAC)).toEqual({ os: "darwin", arch: undefined, mobile: false });
  });

  it("takes the processor from client hints", () => {
    expect(visitorFrom({ platform: "macOS", architecture: "arm", mobile: false }, MAC).arch).toBe("aarch64");
  });

  it("reads an Arm Windows PC from its user agent", () => {
    expect(visitorFrom(undefined, WINDOWS_ARM)).toEqual({ os: "windows", arch: "aarch64", mobile: false });
  });

  it("offers nothing to a Chromebook", () => {
    expect(visitorFrom(undefined, CHROMEBOOK).os).toBeUndefined();
  });
});

describe("offeredPlatforms", () => {
  it("offers a Mac of unknown processor both builds, Apple silicon first", () => {
    expect(offeredPlatforms({ os: "darwin", arch: undefined, mobile: false })).toEqual([
      "darwin-aarch64",
      "darwin-x86_64",
    ]);
  });

  it("offers a phone nothing", () => {
    expect(offeredPlatforms({ os: undefined, arch: undefined, mobile: true })).toEqual([]);
  });
});

describe("cardFiles", () => {
  it("keeps each platform's first installer: the .dmg, the setup .exe and the AppImage", () => {
    expect(new Set(cardFiles(BETA.files).map((file) => file.kind))).toEqual(new Set(["dmg", "nsis", "appimage"]));
  });
});

describe("tableOrder", () => {
  it("lists macOS, Windows then Linux, Apple silicon and x64 first", () => {
    expect(tableOrder(BETA.files).map((file) => file.platform)).toEqual([
      "darwin-aarch64",
      "darwin-x86_64",
      "windows-x86_64",
      "windows-x86_64",
      "windows-aarch64",
      "windows-aarch64",
      "linux-x86_64",
      "linux-x86_64",
      "linux-x86_64",
      "linux-aarch64",
      "linux-aarch64",
      "linux-aarch64",
    ]);
  });
});

describe("offered", () => {
  const beta = JSON.parse(FIXTURE).beta as Downloads;

  it("holds macOS back from the beta, its files and its checksums", () => {
    const list = offered(beta, "beta");
    expect([
      list.files.some((file) => file.platform.startsWith("darwin")),
      Object.keys(list.checksums).some((platform) => platform.startsWith("darwin")),
    ]).toEqual([false, false]);
  });

  it("keeps every Windows and Linux file", () => {
    const own = (files: readonly DownloadFile[]) => files.filter((file) => !file.platform.startsWith("darwin"));
    expect(offered(beta, "beta").files).toEqual(own(beta.files));
  });

  it("offers a nightly's macOS files", () => {
    const nightly = JSON.parse(FIXTURE).nightly as Downloads;
    expect(offered(nightly, "nightly").files).toEqual(nightly.files);
  });
});
