// The Worker as Cloudflare runs it: src/index.ts bundled by esbuild and run in workerd through
// Miniflare, with the three R2 buckets bound and the real Cache API. The handler's branches are
// tested in Node (handler.test.ts); this checks the entry's wiring, the runtime's types, and that
// nothing workerd writes carries the install ID.
import { build } from "esbuild";
import { convertV4MiniflareOptions, Miniflare } from "miniflare";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { cohortIds, fixture } from "./helpers.ts";

let mf: Miniflare;
const logged: string[] = [];

beforeAll(async () => {
  const bundle = await build({
    entryPoints: [new URL("../src/index.ts", import.meta.url).pathname],
    bundle: true,
    format: "esm",
    platform: "neutral",
    target: "es2022",
    write: false,
  });
  const script = bundle.outputFiles[0]?.text;
  if (script === undefined) throw new Error("esbuild produced no output");
  // Miniflare 5 takes a new option shape; this converter takes the documented one.
  mf = new Miniflare(
    convertV4MiniflareOptions({
      modules: true,
      script,
      // wrangler.jsonc's date.
      compatibilityDate: "2026-11-01",
      r2Buckets: ["RELEASES", "STAGING", "NIGHTLY"],
      handleStructuredLogs: (log) => logged.push(JSON.stringify(log)),
    }),
  );
  await seed("RELEASES", "beta", JSON.stringify(fixture("beta")));
  await seed("RELEASES", "stable", '{"schema":1,"channel":"stable","paused":"no"}');
  await seed("STAGING", "staging", JSON.stringify(fixture("staging")));
  await seed("NIGHTLY", "nightly", JSON.stringify(fixture("nightly")));
}, 60_000);

afterAll(async () => {
  await mf?.dispose();
});

/** Writes a channel's manifest into a bucket of the running workerd. */
async function seed(binding: string, channel: string, text: string): Promise<void> {
  // getR2Bucket's type comes from @cloudflare/workers-types, which this package doesn't install;
  // `put` is all the test needs.
  const bucket = (await mf.getR2Bucket(binding)) as unknown as { put(key: string, value: string): Promise<unknown> };
  await bucket.put(`channels/${channel}.json`, text);
}

function get(path: string, installId?: string, method = "GET") {
  return mf.dispatchFetch(`https://updates.laminapdf.com${path}`, {
    method,
    headers: installId ? { "Lamina-Install-Id": installId } : {},
  });
}

describe("the Worker in workerd", () => {
  it("answers /healthz", async () => {
    const response = await get("/healthz");
    expect(response.status).toBe(200);
    expect(await response.text()).toBe("ok\n");
  });

  it("serves the staging channel from the staging bucket", async () => {
    const response = await get("/staging/darwin/aarch64/0.0.1", crypto.randomUUID());
    expect(response.status).toBe(200);
    expect(await response.json()).toMatchObject({
      version: "0.0.2",
      url: "https://staging-downloads.laminapdf.com/releases/0.0.2/darwin-aarch64/Lamina.PDF.app.tar.gz",
    });
  });

  it("serves the nightly channel from the nightly bucket, pre-release versions included", async () => {
    const response = await get("/nightly/linux/aarch64/0.2.1-nightly.20270115.3");
    expect(await response.json()).toMatchObject({ version: "0.2.1-nightly.20270116.7" });
  });

  it("applies the rollout with the header and serves the previous release without it", async () => {
    const { inside } = await cohortIds("0.2.0", 10);
    expect(await (await get("/beta/darwin/aarch64/0.1.3", inside)).json()).toMatchObject({ version: "0.2.0" });
    expect(await (await get("/beta/darwin/aarch64/0.1.0")).json()).toMatchObject({ version: "0.1.3" });
    // Again, now from the Cache API's copy.
    expect(await (await get("/beta/darwin/aarch64/0.1.3", inside)).json()).toMatchObject({ version: "0.2.0" });
  });

  it("answers 204, 400 and 404 with no-store", async () => {
    for (const [path, status] of [
      ["/beta/darwin/aarch64/0.2.0", 204],
      ["/beta/darwin/arm64/0.1.0", 400],
      ["/canary/darwin/aarch64/0.1.0", 404],
    ] as const) {
      const response = await get(path);
      expect(response.status).toBe(status);
      expect(response.headers.get("Cache-Control")).toBe("no-store");
      await response.arrayBuffer();
    }
  });

  it("logs a broken manifest without the install ID", async () => {
    const id = crypto.randomUUID();
    const response = await get("/stable/darwin/aarch64/0.1.0", id);
    expect(response.status).toBe(204);
    await response.arrayBuffer();
    // workerd hands its logs over asynchronously.
    await expect.poll(() => logged.some((line) => line.includes("manifest-invalid")), { timeout: 5_000 }).toBe(true);
    expect(logged.join("\n")).not.toContain(id);
    expect(logged.join("\n").toLowerCase()).not.toContain(id.toLowerCase());
  });
});
