// The Worker as Cloudflare runs it: src/index.ts bundled by esbuild and run in workerd through
// Miniflare, with wrangler.jsonc's R2 buckets and rate limiter bound and the real Cache API. The
// handler's branches are tested in Node (handler.test.ts, crash.test.ts); this checks the entry's
// wiring, the runtime's types, and that nothing workerd writes carries the install ID.
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { convertV4MiniflareOptions, Miniflare } from "miniflare";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { cohortIds, crashFixture, fixture } from "./helpers.ts";
import { wranglerConfig } from "./wrangler-config.ts";

let mf: Miniflare;
const logged: string[] = [];

beforeAll(async () => {
  const bundle = await build({
    entryPoints: [fileURLToPath(new URL("../src/index.ts", import.meta.url))],
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
      compatibilityDate: wranglerConfig().compatibility_date,
      r2Buckets: wranglerConfig().r2_buckets.map((bucket) => bucket.binding),
      ratelimits: Object.fromEntries(wranglerConfig().ratelimits.map(({ name, ...limit }) => [name, limit])),
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

// getR2Bucket's type comes from @cloudflare/workers-types, which this package doesn't install;
// `put` and `get` are all the tests need.
interface Bucket {
  put(key: string, value: string): Promise<unknown>;
  get(key: string): Promise<{ text(): Promise<string>; httpMetadata?: { contentType?: string } } | null>;
}

async function bucket(binding: string): Promise<Bucket> {
  return (await mf.getR2Bucket(binding)) as unknown as Bucket;
}

/** Writes a channel's manifest into a bucket of the running workerd. */
async function seed(binding: string, channel: string, text: string): Promise<void> {
  await (await bucket(binding)).put(`channels/${channel}.json`, text);
}

function get(path: string, installId?: string, method = "GET") {
  return mf.dispatchFetch(`https://updates.laminapdf.com${path}`, {
    method,
    headers: installId ? { "Lamina-Install-Id": installId } : {},
  });
}

// The unit is the Worker's default export, as workerd calls it.
describe("fetch", () => {
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

  it("stores a crash report in the crash bucket and answers 202", async () => {
    const report = crashFixture();
    const dayOf = () => new Date().toISOString().slice(0, 10).replaceAll("-", "/");
    const before = dayOf();
    const response = await mf.dispatchFetch("https://updates.laminapdf.com/crash", {
      method: "POST",
      headers: { "Content-Type": "application/json", "CF-Connecting-IP": "192.0.2.9" },
      body: JSON.stringify(report),
    });
    expect(response.status).toBe(202);
    expect(await response.text()).toBe("");
    const keyOn = (day: string) => `crash-reports/${day}/${String(report.report_id)}.json`;
    // The Worker's clock's UTC day: the one before the request unless midnight passed during it.
    const crashes = await bucket("CRASH_REPORTS");
    const stored = (await crashes.get(keyOn(before))) ?? (await crashes.get(keyOn(dayOf())));
    expect(JSON.parse((await stored?.text()) ?? "null")).toEqual(report);
    expect(stored?.httpMetadata?.contentType).toBe("application/json");
    expect(await (await bucket("RELEASES")).get(keyOn(before))).toBeNull();
  });

  it("limits crash reports to 10 a minute per IP", async () => {
    // The limiter counts in fixed one-minute windows, so a run that crosses a minute boundary
    // starts a second window part-way through. 25 reports always reach the limit in at least one
    // window (at most two windows hold 10 each), however the boundary falls.
    const statuses: number[] = [];
    for (let i = 0; i < 25; i += 1) {
      const response = await mf.dispatchFetch("https://updates.laminapdf.com/crash", {
        method: "POST",
        headers: { "Content-Type": "application/json", "CF-Connecting-IP": "192.0.2.10" },
        body: JSON.stringify(crashFixture()),
      });
      statuses.push(response.status);
      await response.arrayBuffer();
    }
    expect(statuses[0]).toBe(202);
    expect(statuses.every((status) => status === 202 || status === 429)).toBe(true);
    expect(statuses).toContain(429);
    expect(statuses.filter((status) => status === 202).length).toBeLessThanOrEqual(20);
  });
});
