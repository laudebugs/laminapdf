// R2's S3 client (scripts/r2.ts) against a recording fetch: which credentials each bucket takes,
// the S3 secret of a token, the addresses and headers it signs, and how it reads answers.
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { credentialsFor, objectUrl, R2, secretOf } from "./r2.ts";

const ACCOUNT = "0123456789abcdef0123456789abcdef";
const CREDENTIALS = { accountId: ACCOUNT, accessKeyId: "key-id", token: "token-value" };

interface Sent {
  readonly url: string;
  readonly method: string;
  readonly headers: Headers;
  readonly body: string;
}

/** An R2 whose requests are recorded in `sent` and answered with `status`. */
function recording(status = 200, body = ""): { r2: R2; sent: Sent[] } {
  const sent: Sent[] = [];
  const fetcher = (async (input: string | URL | Request, init?: RequestInit) => {
    const request = new Request(input, init);
    sent.push({
      url: request.url,
      method: request.method,
      headers: request.headers,
      body: init?.body === undefined ? "" : await request.text(),
    });
    return new Response(status === 204 || init?.method === "HEAD" ? null : body, { status });
  }) as typeof fetch;
  return { r2: new R2(CREDENTIALS, fetcher), sent };
}

describe("credentialsFor", () => {
  it("takes the staging bucket's own token and key ID", () => {
    const env = {
      CLOUDFLARE_ACCOUNT_ID: ACCOUNT,
      CLOUDFLARE_STAGING_R2_TOKEN: "staging-token",
      STAGING_R2_ACCESS_KEY_ID: "staging-id",
      CLOUDFLARE_API_TOKEN: "other",
      R2_ACCESS_KEY_ID: "other-id",
    };
    expect(credentialsFor("lamina-pdf-staging", env)).toEqual({
      accountId: ACCOUNT,
      accessKeyId: "staging-id",
      token: "staging-token",
    });
  });

  it("takes the job's token and key ID for another bucket", () => {
    const env = { CLOUDFLARE_ACCOUNT_ID: ACCOUNT, CLOUDFLARE_API_TOKEN: "t", R2_ACCESS_KEY_ID: "id" };
    expect(credentialsFor("lamina-pdf-releases", env).accessKeyId).toBe("id");
  });

  it("names what is missing, never a value", () => {
    expect(() => credentialsFor("lamina-pdf-staging", { CLOUDFLARE_STAGING_R2_TOKEN: "secret" })).toThrow(
      "lamina-pdf-staging needs CLOUDFLARE_ACCOUNT_ID, STAGING_R2_ACCESS_KEY_ID in the environment",
    );
  });
});

describe("secretOf", () => {
  it("is the SHA-256 of the token in hex", () => {
    expect(secretOf("abc")).toBe("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  });
});

describe("objectUrl", () => {
  it("addresses the account's S3 endpoint with each segment encoded", () => {
    expect(objectUrl(ACCOUNT, "lamina-pdf-staging", "releases/0.2.0/Lamina PDF.dmg")).toBe(
      `https://${ACCOUNT}.r2.cloudflarestorage.com/lamina-pdf-staging/releases/0.2.0/Lamina%20PDF.dmg`,
    );
  });
});

describe("R2", () => {
  it("signs a put for the key ID, R2's region and S3, without hashing the body", async () => {
    const { r2, sent } = recording();
    await r2.putObject("lamina-pdf-staging", "channels/staging.json", "{}", { contentType: "application/json" });
    const authorization = sent[0]?.headers.get("authorization") ?? "";
    expect([
      authorization.startsWith("AWS4-HMAC-SHA256 Credential=key-id/"),
      authorization.includes("/auto/s3/aws4_request"),
      sent[0]?.headers.get("x-amz-content-sha256"),
    ]).toEqual([true, true, "UNSIGNED-PAYLOAD"]);
  });

  it("stores a put's body with its type and cache headers", async () => {
    const { r2, sent } = recording();
    await r2.putObject("lamina-pdf-staging", "channels/staging.json", "{}", {
      contentType: "application/json",
      cacheControl: "no-store",
    });
    expect([
      sent[0]?.method,
      sent[0]?.body,
      sent[0]?.headers.get("content-type"),
      sent[0]?.headers.get("cache-control"),
    ]).toEqual(["PUT", "{}", "application/json", "no-store"]);
  });

  it("streams a file with its length", async () => {
    const file = join(mkdtempSync(join(tmpdir(), "lamina-r2-")), "bundle.bin");
    writeFileSync(file, "bundle bytes");
    const { r2, sent } = recording();
    await r2.putObject("lamina-pdf-staging", "e2e/bundle.bin", { file }, { contentType: "application/octet-stream" });
    expect([sent[0]?.body, sent[0]?.headers.get("content-length")]).toEqual(["bundle bytes", "12"]);
  });

  it("reads a missing object as undefined", async () => {
    const { r2 } = recording(404);
    expect(await r2.getObject("lamina-pdf-staging", "channels/beta.json")).toBeUndefined();
  });

  it("reads an object's bytes", async () => {
    const { r2 } = recording(200, "{}");
    expect(new TextDecoder().decode(await r2.getObject("lamina-pdf-staging", "channels/staging.json"))).toBe("{}");
  });

  it("fails with the status of a refused request", async () => {
    const { r2 } = recording(403, "AccessDenied");
    await expect(r2.deleteObject("lamina-pdf-releases", "x")).rejects.toThrow(
      "DELETE lamina-pdf-releases/x answered 403: AccessDenied",
    );
  });

  it("reads a missing object's head as undefined", async () => {
    const { r2 } = recording(404);
    expect(await r2.headObject("lamina-pdf-staging", "nothing")).toBeUndefined();
  });
});
