import { describe, expect, it } from "vitest";
import { type PlatformFiles, releaseFrom, updaterBundle } from "./release.ts";

const input = {
  version: "0.1.1-nightly.20261028.42",
  pubDate: "2026-10-28T03:00:00Z",
  notes: "Nightly build of main at 0123abc",
  downloads: "https://nightly-downloads.laminapdf.com",
};

function built(platform: string, files: string[]): PlatformFiles {
  return { platform, files, read: (file) => `signature of ${file}\n` };
}

describe("updaterBundle", () => {
  it("picks what each OS's updater installs", () => {
    const mac = ["Lamina PDF_0.1.1_aarch64.dmg", "Lamina PDF.app.tar.gz", "Lamina PDF.app.tar.gz.sig"];
    expect(updaterBundle("darwin-aarch64", mac)).toBe("Lamina PDF.app.tar.gz");
    const windows = [
      "Lamina PDF_0.1.1_x64-setup.exe",
      "Lamina PDF_0.1.1_x64-setup.exe.sig",
      "Lamina PDF_0.1.1_x64_en-US.msi",
    ];
    expect(updaterBundle("windows-x86_64", windows)).toBe("Lamina PDF_0.1.1_x64-setup.exe");
    const linux = [
      "Lamina PDF_0.1.1_amd64.AppImage",
      "lamina-pdf_0.1.1_amd64.deb",
      "Lamina PDF_0.1.1_amd64.AppImage.sig",
    ];
    expect(updaterBundle("linux-x86_64", linux)).toBe("Lamina PDF_0.1.1_amd64.AppImage");
  });

  it("refuses two candidates", () => {
    expect(() => updaterBundle("linux-x86_64", ["a.AppImage", "b.AppImage"])).toThrow(/more than one/);
  });
});

describe("releaseFrom", () => {
  it("names each platform's bundle under the version's folder, with its signature", () => {
    const release = releaseFrom(input, [
      built("linux-aarch64", ["Lamina PDF_aarch64.AppImage", "Lamina PDF_aarch64.AppImage.sig"]),
    ]);
    expect(release).toEqual({
      version: input.version,
      pub_date: input.pubDate,
      notes: input.notes,
      platforms: {
        "linux-aarch64": {
          url: "https://nightly-downloads.laminapdf.com/releases/0.1.1-nightly.20261028.42/linux-aarch64/Lamina%20PDF_aarch64.AppImage",
          signature: "signature of Lamina PDF_aarch64.AppImage.sig",
        },
      },
    });
  });

  it("stops when a platform has no bundle or no signature", () => {
    expect(() => releaseFrom(input, [built("darwin-aarch64", ["x.dmg"])])).toThrow(/no updater bundle/);
    expect(() => releaseFrom(input, [built("darwin-aarch64", ["Lamina PDF.app.tar.gz"])])).toThrow(/no .sig/);
    expect(() => releaseFrom(input, [built("solaris-sparc", [])])).toThrow(/platform key/);
    expect(() => releaseFrom(input, [])).toThrow(/no platform/);
  });
});
