// The platform packages tools/cli-release generates against dist-workspace.toml's targets: each target
// has one package whose `os`, `cpu` and `libc` are what npm needs to install it on that target
// only, worked out here from the target triple itself.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { platformManifest, platforms, unpackCommand, wrapperManifest } from "../../../tools/cli-release/generate.ts";

const ROOT = new URL("../../../", import.meta.url);

/** dist-workspace.toml's `targets`. */
function distTargets(): string[] {
  const config = readFileSync(new URL("dist-workspace.toml", ROOT), "utf8");
  const list = /^targets = \[([^\]]*)\]/m.exec(config)?.[1] ?? "";
  return [...list.matchAll(/"([^"]+)"/g)].map((m) => m[1] ?? "");
}

/** What npm must see for a Rust target triple. */
function expected(target: string): Record<string, string[]> {
  const cpu = target.startsWith("aarch64-") ? "arm64" : target.startsWith("x86_64-") ? "x64" : "?";
  if (target.endsWith("-apple-darwin")) return { os: ["darwin"], cpu: [cpu] };
  if (target.endsWith("-pc-windows-msvc")) return { os: ["win32"], cpu: [cpu] };
  const libc = target.endsWith("-linux-musl") ? "musl" : target.endsWith("-linux-gnu") ? "glibc" : "?";
  return { os: ["linux"], cpu: [cpu], libc: [libc] };
}

describe("the platform packages", () => {
  it("cover dist's targets, one package each", () => {
    expect(
      platforms()
        .map((p) => p.target)
        .sort(),
    ).toEqual(distTargets().sort());
  });

  it.each(distTargets())("declare the os, cpu and libc of %s", (target) => {
    const platform = platforms().find((p) => p.target === target);
    expect(platform).toBeDefined();
    if (platform === undefined) return;
    const manifest = platformManifest(platform, "1.2.3");
    const { os, cpu, libc } = manifest;
    expect({ os, cpu, ...(libc === undefined ? {} : { libc }) }).toEqual(expected(target));
    expect(manifest.version).toBe("1.2.3");
  });
});

describe("the wrapper", () => {
  const base = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as Record<string, unknown>;
  const wrapper = wrapperManifest(base, platforms(), "1.2.3");

  it("depends on every platform package at its own version", () => {
    expect(wrapper.optionalDependencies).toEqual(Object.fromEntries(platforms().map((p) => [p.package, "1.2.3"])));
  });

  it("is publishable and carries no workspace-only fields", () => {
    expect(wrapper).not.toHaveProperty("private");
    expect(wrapper).not.toHaveProperty("devDependencies");
    expect(wrapper).not.toHaveProperty("scripts");
    expect(wrapper.version).toBe("1.2.3");
  });

  it("installs only on the operating systems and CPUs the platform packages cover", () => {
    expect(new Set(base.os as string[])).toEqual(new Set(platforms().map((p) => p.os)));
    expect(new Set(base.cpu as string[])).toEqual(new Set(platforms().map((p) => p.cpu)));
  });
});

describe("unpackCommand", () => {
  it("sends a Windows zip to unzip on Linux, whose GNU tar reads no zip", () => {
    expect(unpackCommand("a.zip", "out", "linux")).toEqual(["unzip", ["-q", "a.zip", "-d", "out"]]);
  });

  it.each(["darwin", "win32"])("leaves a zip to bsdtar on %s", (host) => {
    expect(unpackCommand("a.zip", "out", host)).toEqual(["tar", ["-xf", "a.zip", "-C", "out"]]);
  });

  it("unpacks .tar.xz with tar everywhere", () => {
    expect(unpackCommand("a.tar.xz", "out", "linux")).toEqual(["tar", ["-xf", "a.tar.xz", "-C", "out"]]);
  });
});
