import { copyFile, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { searchRules } from "../plugins/search.ts";

const HEADERS = new URL("../public/_headers", import.meta.url);
let dir = "";

/** What a build for `environment` writes: robots.txt (emitted with the bundle) and dist/_headers. */
async function built(environment: string | undefined): Promise<{ robots: string; headers: string }> {
  dir = await mkdtemp(join(tmpdir(), "lamina-web-search-"));
  await copyFile(HEADERS, join(dir, "_headers"));
  const plugin = searchRules(environment);
  let robots = "";
  const call = <A extends unknown[]>(hook: unknown, self: unknown, ...args: A) =>
    (hook as (...a: A) => unknown).call(self, ...args);
  call(plugin.configResolved, undefined, { root: "/", build: { outDir: dir } });
  call(plugin.generateBundle, { emitFile: (file: { source: string }) => (robots = file.source) });
  await call(plugin.closeBundle, undefined);
  return { robots, headers: await readFile(join(dir, "_headers"), "utf8") };
}

afterEach(async () => {
  if (dir !== "") await rm(dir, { recursive: true, force: true });
});

describe("what the reader tells search engines", () => {
  it("production lets crawlers in, names no sitemap, and leaves _headers as public/_headers", async () => {
    const { robots, headers } = await built("production");
    expect(robots).toBe("User-agent: *\nAllow: /\n");
    expect(headers).toBe(await readFile(HEADERS, "utf8"));
  });

  it.each([["staging"], [undefined]])(
    "%s lets crawlers in, and adds only the noindex line to public/_headers",
    async (environment) => {
      const { robots, headers } = await built(environment);
      expect(robots).toBe("User-agent: *\nAllow: /\n");
      const source = await readFile(HEADERS, "utf8");
      expect(headers).toBe(source.replace(/^\/\*$/m, "/*\n  X-Robots-Tag: noindex"));
    },
  );
});

const INDEX = await readFile(new URL("../index.html", import.meta.url), "utf8");

describe("the reader's canonical link and share tags", () => {
  const html = INDEX;
  const tags = (environment: string | undefined) => {
    const hook = searchRules(environment).transformIndexHtml as unknown as (
      html: string,
    ) => { attrs: Record<string, string> }[];
    return hook(html).map(({ attrs }) => attrs);
  };

  it("production names / as canonical and describes the page from its own title and description", () => {
    const attrs = tags("production");
    expect(attrs[0]).toEqual({ rel: "canonical", href: "https://app.laminapdf.com/" });
    const content = (name: string) => attrs.find((a) => a.property === name || a.name === name)?.content;
    expect(content("og:title")).toBe("Lamina PDF");
    expect(content("og:description")).toBe(html.match(/<meta name="description" content="([^"]+)"/)?.[1]);
    expect(content("og:url")).toBe("https://app.laminapdf.com/");
    expect(content("og:image")).toBe("https://laminapdf.com/og.png");
    expect(content("twitter:card")).toBe("summary_large_image");
  });

  it.each([["staging"], [undefined]])("%s adds no canonical link or share tag", (environment) => {
    expect(tags(environment)).toEqual([]);
  });

  it("fails on a page without a description, rather than share an empty card", () => {
    const hook = searchRules("production").transformIndexHtml as unknown as (html: string) => unknown;
    expect(() => hook("<title>Lamina PDF</title>")).toThrow("needs a <title> and a description");
  });
});
