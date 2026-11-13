import { beforeEach, describe, expect, it } from "vitest";
import { MANIFEST_TTL_MS } from "../src/cache.ts";
import { handle } from "../src/handler.ts";
import type { Manifest } from "../src/manifest.ts";
import { cohortIds, fixture, type Harness, harness, request } from "./helpers.ts";

let h: Harness;
const beta = fixture("beta");

beforeEach(() => {
  h = harness();
  h.store.put("beta", beta);
});

async function check(path: string, init?: Parameters<typeof request>[1]): Promise<Response> {
  const response = await handle(request(path, init), h.deps);
  await h.settle();
  return response;
}

// The fixture as JSON the tests may change.
interface Editable {
  paused: boolean;
  release: { rollout_percent: number };
  bridges: { below: string; release: { version: string } }[];
}

/** The beta manifest with its release at 100 %, so the rollout doesn't decide. */
function everyone(edit: (m: Editable) => void = () => {}): Manifest {
  const m = structuredClone(beta) as unknown as Editable;
  m.release.rollout_percent = 100;
  edit(m);
  return m as unknown as Manifest;
}

describe("handle", () => {
  describe("the update endpoint", () => {
    it("answers 200 with Tauri's dynamic JSON when a newer release exists", async () => {
      h.store.put("beta", everyone());
      const response = await check("/beta/darwin/aarch64/0.1.3");
      expect(response.status).toBe(200);
      expect(response.headers.get("Content-Type")).toBe("application/json");
      expect(response.headers.get("Cache-Control")).toBe("no-store");
      expect(response.headers.get("X-Content-Type-Options")).toBe("nosniff");
      expect(await response.json()).toEqual({
        version: "0.2.0",
        pub_date: "2027-01-15T10:00:00Z",
        url: "https://downloads.laminapdf.com/releases/0.2.0/darwin-aarch64/Lamina.PDF.app.tar.gz",
        signature: beta.release?.platforms["darwin-aarch64"]?.signature,
        notes: "https://laminapdf.com/desktop/releases/0.2.0",
      });
    });

    it("answers 204 when the install is up to date", async () => {
      h.store.put("beta", everyone());
      const response = await check("/beta/darwin/aarch64/0.2.0");
      expect(response.status).toBe(204);
      expect(response.headers.get("Cache-Control")).toBe("no-store");
      expect(await response.text()).toBe("");
    });

    it("answers 204 to an install newer than every release", async () => {
      h.store.put("beta", everyone());
      expect((await check("/beta/darwin/aarch64/0.3.0")).status).toBe(204);
    });

    it("answers 204 when the release has no bundle for the platform", async () => {
      h.store.put("beta", everyone());
      expect((await check("/beta/linux/aarch64/0.1.3")).status).toBe(204);
    });

    it("answers 204 for a channel with no manifest yet", async () => {
      expect((await check("/stable/darwin/aarch64/0.1.0")).status).toBe(204);
    });

    it("answers HEAD with the status and no body", async () => {
      h.store.put("beta", everyone());
      const response = await check("/beta/darwin/aarch64/0.1.3", { method: "HEAD" });
      expect(response.status).toBe(200);
      expect(await response.text()).toBe("");
    });
  });

  describe("the rollout", () => {
    it("serves the release to an install inside its cohort", async () => {
      const { inside } = await cohortIds("0.2.0", 10);
      const response = await check("/beta/darwin/aarch64/0.1.3", { installId: inside });
      expect(await response.json()).toMatchObject({ version: "0.2.0" });
    });

    it("serves the same answer to the same install every time", async () => {
      const { inside, outside } = await cohortIds("0.2.0", 10);
      for (let i = 0; i < 3; i += 1) {
        expect(await (await check("/beta/darwin/aarch64/0.1.3", { installId: inside })).json()).toMatchObject({
          version: "0.2.0",
        });
        expect((await check("/beta/darwin/aarch64/0.1.4", { installId: outside })).status).toBe(204);
      }
    });

    it("reads the install ID in any case", async () => {
      const { inside } = await cohortIds("0.2.0", 10);
      const response = await check("/beta/darwin/aarch64/0.1.3", { installId: inside.toUpperCase() });
      expect(await response.json()).toMatchObject({ version: "0.2.0" });
    });

    it("falls back to the previous release outside the cohort", async () => {
      const { outside } = await cohortIds("0.2.0", 10);
      const response = await check("/beta/darwin/aarch64/0.1.0", { installId: outside });
      expect(await response.json()).toMatchObject({ version: "0.1.3" });
    });

    it("answers 204 outside the cohort when the previous release is no newer", async () => {
      const { outside } = await cohortIds("0.2.0", 10);
      expect((await check("/beta/darwin/aarch64/0.1.3", { installId: outside })).status).toBe(204);
    });

    it("serves only 100 % releases to a request without the header", async () => {
      const response = await check("/beta/darwin/aarch64/0.1.0");
      expect(await response.json()).toMatchObject({ version: "0.1.3" });
    });

    it("treats a malformed header as absent", async () => {
      const response = await check("/beta/darwin/aarch64/0.1.0", { installId: "not-a-uuid" });
      expect(await response.json()).toMatchObject({ version: "0.1.3" });
    });
  });

  describe("bridges", () => {
    const bridge = beta.bridges?.[0];

    it("serves the bridge's own url and signature below its bound", async () => {
      const response = await check("/beta/darwin/aarch64/0.0.9");
      expect(await response.json()).toEqual({
        version: "0.1.0",
        pub_date: "2026-11-20T10:00:00Z",
        url: bridge?.release.platforms["darwin-aarch64"]?.url,
        signature: bridge?.release.platforms["darwin-aarch64"]?.signature,
        notes: "https://laminapdf.com/desktop/releases/0.1.0",
      });
    });

    it("doesn't serve the bridge at its bound", async () => {
      h.store.put("beta", everyone());
      expect(await (await check("/beta/darwin/aarch64/0.1.0")).json()).toMatchObject({ version: "0.2.0" });
    });

    it("serves the oldest bridge an install needs first", async () => {
      h.store.put(
        "beta",
        everyone((m) => {
          const later = structuredClone(m.bridges[0]);
          if (!later) throw new Error("the beta fixture has a bridge");
          later.below = "0.1.2";
          later.release.version = "0.1.2";
          m.bridges.unshift(later);
        }),
      );
      expect(await (await check("/beta/darwin/aarch64/0.0.5")).json()).toMatchObject({ version: "0.1.0" });
      expect(await (await check("/beta/darwin/aarch64/0.1.1")).json()).toMatchObject({ version: "0.1.2" });
    });
  });

  describe("the kill switch", () => {
    it("answers 204 to installs outside every bridge while paused", async () => {
      h.store.put(
        "beta",
        everyone((m) => {
          m.paused = true;
        }),
      );
      expect((await check("/beta/darwin/aarch64/0.1.3")).status).toBe(204);
      expect((await check("/beta/darwin/aarch64/0.1.0")).status).toBe(204);
    });

    it("still serves the bridge while paused", async () => {
      h.store.put(
        "beta",
        everyone((m) => {
          m.paused = true;
        }),
      );
      expect(await (await check("/beta/darwin/aarch64/0.0.9")).json()).toMatchObject({ version: "0.1.0" });
    });

    it("takes effect within 30 seconds, with no second cache layer", async () => {
      h.store.put("beta", everyone());
      // t = 0: the manifest is read and cached.
      expect((await check("/beta/darwin/aarch64/0.1.3")).status).toBe(200);
      // t = 1 s: the owner pauses the channel in R2.
      h.advance(1_000);
      h.store.put(
        "beta",
        everyone((m) => {
          m.paused = true;
        }),
      );
      expect((await check("/beta/darwin/aarch64/0.1.3")).status).toBe(200);
      // Just before the TTL ends the cached copy still answers; from 30 s on, R2's does.
      h.advance(MANIFEST_TTL_MS - 1_000 - 1);
      expect((await check("/beta/darwin/aarch64/0.1.3")).status).toBe(200);
      h.advance(1);
      expect((await check("/beta/darwin/aarch64/0.1.3")).status).toBe(204);
    });

    it("reads R2 once per TTL, not once per request", async () => {
      for (let i = 0; i < 5; i += 1) await check("/beta/darwin/aarch64/0.1.3");
      expect(h.store.reads).toBe(1);
      h.advance(MANIFEST_TTL_MS);
      await check("/beta/darwin/aarch64/0.1.3");
      expect(h.store.reads).toBe(2);
    });

    it("caches a missing manifest too", async () => {
      await check("/stable/darwin/aarch64/0.1.0");
      await check("/stable/darwin/aarch64/0.1.0");
      expect(h.store.reads).toBe(1);
    });
  });

  describe("bad requests", () => {
    it.each([
      "/beta/macos/aarch64/0.1.0",
      "/beta/darwin/arm64/0.1.0",
      "/beta/darwin/aarch64/v0.1.0",
      "/beta/darwin/aarch64/0.1",
      "/beta/darwin/aarch64/%20",
    ])("answers 400 to %s", async (path) => {
      expect((await check(path)).status).toBe(400);
    });

    it.each(["/canary/darwin/aarch64/0.1.0", "/", "/beta", "/beta/darwin/aarch64/0.1.0/extra", "/favicon.ico"])(
      "answers 404 to %s",
      async (path) => {
        expect((await check(path)).status).toBe(404);
      },
    );

    it("answers 405 to other methods", async () => {
      const response = await check("/beta/darwin/aarch64/0.1.0", { method: "POST" });
      expect(response.status).toBe(405);
      expect(response.headers.get("Allow")).toBe("GET, HEAD");
    });

    it("answers /healthz", async () => {
      const response = await check("/healthz");
      expect(response.status).toBe(200);
      expect(await response.text()).toBe("ok\n");
    });
  });

  describe("a broken manifest", () => {
    it("offers nothing and logs which channel and why", async () => {
      h.store.put("beta", '{"schema":1,"channel":"beta","paused":"no"}');
      expect((await check("/beta/darwin/aarch64/0.1.0")).status).toBe(204);
      expect(h.logs).toEqual([
        { event: "manifest-invalid", channel: "beta", errors: ["/paused: must be true or false"] },
      ]);
    });

    it("answers 503 when R2 fails, and logs it", async () => {
      h.deps.stores.beta.get = async () => {
        throw new Error("R2 unavailable");
      };
      expect((await check("/beta/darwin/aarch64/0.1.0")).status).toBe(503);
      expect(h.logs).toEqual([
        { event: "manifest-unavailable", channel: "beta", error: { name: "Error", message: "R2 unavailable" } },
      ]);
    });
  });
});
