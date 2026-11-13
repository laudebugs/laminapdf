// Sitemap dates from git: each entry dated by its sources' last commit, and a production build
// refusing a shallow checkout rather than write wrong dates. The git cases run on a small
// repository made for the test, and a shallow clone of it, whatever this checkout's depth.

import { execFileSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { env as environment } from "node:process";
import { pathToFileURL } from "node:url";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { datesAvailable, isShallow, lastCommitDate, withLastmod } from "../src/lastmod.mjs";

describe("withLastmod", () => {
  const xml =
    "<urlset><url><loc>https://a.example/</loc></url><url><loc>https://a.example/b/</loc></url>" +
    "<url><loc>https://a.example/c/</loc><lastmod>2026-01-01</lastmod></url></urlset>";

  it("dates the entries it has a date for, and leaves the others and existing dates alone", () => {
    const dates: Record<string, string> = { "https://a.example/": "2026-11-12T10:00:00Z", "https://a.example/c/": "x" };
    expect(withLastmod(xml, (loc) => dates[loc])).toBe(
      "<urlset><url><loc>https://a.example/</loc><lastmod>2026-11-12T10:00:00Z</lastmod></url>" +
        "<url><loc>https://a.example/b/</loc></url>" +
        "<url><loc>https://a.example/c/</loc><lastmod>2026-01-01</lastmod></url></urlset>",
    );
  });
});

describe("dates from git", () => {
  let dir = "";
  let full: URL;
  let shallow: URL;

  /** Commits `file` with `text` at `date` in the repository at `cwd`. */
  async function commit(cwd: string, file: string, text: string, date: string): Promise<void> {
    await writeFile(join(cwd, file), text);
    const env = { ...environment, GIT_AUTHOR_DATE: date, GIT_COMMITTER_DATE: date };
    execFileSync("git", ["add", file], { cwd });
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-qm", file], { cwd, env });
  }

  beforeAll(async () => {
    dir = await mkdtemp(join(tmpdir(), "lamina-lastmod-"));
    const repo = join(dir, "repo");
    execFileSync("git", ["init", "-q", repo]);
    await commit(repo, "a.md", "a", "2026-01-02T03:04:05Z");
    await commit(repo, "b.md", "b", "2026-02-03T04:05:06Z");
    execFileSync("git", ["clone", "-q", "--depth", "1", pathToFileURL(repo).href, join(dir, "shallow")]);
    full = pathToFileURL(`${repo}/`);
    shallow = pathToFileURL(`${join(dir, "shallow")}/`);
  });

  afterAll(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("dates each file by the last commit that changed it, and a set by the newest", () => {
    const date = (names: string[]) =>
      lastCommitDate(
        full,
        names.map((name) => new URL(name, full)),
      );
    expect(date(["a.md"])).toBe("2026-01-02T03:04:05+00:00");
    expect(date(["a.md", "b.md"])).toBe("2026-02-03T04:05:06+00:00");
  });

  it("gives no date for a file that was never committed, or for no files", () => {
    expect(lastCommitDate(full, [new URL("never.md", full)])).toBeUndefined();
    expect(lastCommitDate(full, [])).toBeUndefined();
  });

  it("writes dates from a full history, in production or not", () => {
    expect(isShallow(full)).toBe(false);
    expect(datesAvailable(full, true)).toBe(true);
    expect(datesAvailable(full, false)).toBe(true);
  });

  it("refuses a shallow checkout in production, and writes no dates elsewhere", () => {
    expect(isShallow(shallow)).toBe(true);
    expect(() => datesAvailable(shallow, true)).toThrow("fetch-depth: 0");
    expect(datesAvailable(shallow, false)).toBe(false);
  });
});
