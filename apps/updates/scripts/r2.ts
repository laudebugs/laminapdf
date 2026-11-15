// R2's objects through its S3-compatible API (SigV4, with aws4fetch), for the scripts that write
// the desktop releases' buckets: the manifest tool, the desktop update test and the release
// workflow's uploads. Not Wrangler's `r2 object` commands: they call Cloudflare's REST API, which
// an R2 token scoped to one bucket (Object Read & Write, apps/updates/README.md) can't use, and
// such a token is all the workflows hold.
//
// An R2 API token's S3 credentials: the Access Key ID shown with the token, and the SHA-256 of the
// token's value as the secret. Each bucket's token and key ID come from the environment
// (`credentialsFor`), so one job holds only the buckets its tokens name.
//
//   node scripts/r2.ts put <bucket> <key> <file> [--content-type <type>] [--cache-control <value>]
//   node scripts/r2.ts delete <bucket> <key>
import { createHash } from "node:crypto";
import { createReadStream, statSync } from "node:fs";
import process from "node:process";
import { Readable } from "node:stream";
import { pathToFileURL } from "node:url";
import { AwsClient } from "aws4fetch";

/** An R2 token's S3 credentials, on one account. */
export interface R2Credentials {
  readonly accountId: string;
  readonly accessKeyId: string;
  /** The token's value; the S3 secret is its SHA-256. */
  readonly token: string;
}

/** The staging bucket, whose token is apart from the others' in the release workflow. */
const STAGING_BUCKET = "lamina-pdf-staging";

/**
 * The environment's names for `bucket`'s credentials: the staging bucket's are
 * CLOUDFLARE_STAGING_R2_TOKEN and STAGING_R2_ACCESS_KEY_ID; every other bucket's are the job's
 * CLOUDFLARE_API_TOKEN and R2_ACCESS_KEY_ID. CLOUDFLARE_ACCOUNT_ID is shared.
 */
export function credentialNames(bucket: string): { readonly token: string; readonly accessKeyId: string } {
  return bucket === STAGING_BUCKET
    ? { token: "CLOUDFLARE_STAGING_R2_TOKEN", accessKeyId: "STAGING_R2_ACCESS_KEY_ID" }
    : { token: "CLOUDFLARE_API_TOKEN", accessKeyId: "R2_ACCESS_KEY_ID" };
}

/** `bucket`'s credentials from `env`; throws naming what is missing (names only, never values). */
export function credentialsFor(bucket: string, env: Readonly<Record<string, string | undefined>>): R2Credentials {
  const names = credentialNames(bucket);
  const accountId = env.CLOUDFLARE_ACCOUNT_ID;
  const accessKeyId = env[names.accessKeyId];
  const token = env[names.token];
  const missing = [
    accountId ? undefined : "CLOUDFLARE_ACCOUNT_ID",
    accessKeyId ? undefined : names.accessKeyId,
    token ? undefined : names.token,
  ].filter((name) => name !== undefined);
  if (accountId === undefined || accessKeyId === undefined || token === undefined || missing.length > 0) {
    throw new Error(`${bucket} needs ${missing.join(", ")} in the environment`);
  }
  return { accountId, accessKeyId, token };
}

/** The S3 secret of an R2 token: the SHA-256 of its value, in hex. */
export function secretOf(token: string): string {
  return createHash("sha256").update(token).digest("hex");
}

/** The S3 address of `key` in `bucket`, each path segment percent-encoded. */
export function objectUrl(accountId: string, bucket: string, key: string): string {
  const path = key.split("/").map(encodeURIComponent).join("/");
  return `https://${accountId}.r2.cloudflarestorage.com/${encodeURIComponent(bucket)}/${path}`;
}

/** What a stored object says about itself. */
export interface ObjectInfo {
  readonly size: number;
  readonly etag: string | undefined;
  readonly contentType: string | undefined;
}

/** A body to store: text, bytes, or a file read as a stream (an installer needn't sit in memory). */
export type Body = string | Uint8Array | { readonly file: string };

/** The headers a stored object is served with. */
export interface PutOptions {
  readonly contentType: string;
  readonly cacheControl?: string;
}

/** One account's R2, through the S3 API. */
export class R2 {
  readonly #aws: AwsClient;
  readonly #accountId: string;
  readonly #fetch: typeof fetch;

  constructor(credentials: R2Credentials, fetcher: typeof fetch = fetch) {
    this.#accountId = credentials.accountId;
    this.#fetch = fetcher;
    this.#aws = new AwsClient({
      accessKeyId: credentials.accessKeyId,
      secretAccessKey: secretOf(credentials.token),
      service: "s3",
      region: "auto",
    });
  }

  /** The R2 of `bucket`'s credentials in `env`. */
  static for(bucket: string, env: Readonly<Record<string, string | undefined>> = process.env): R2 {
    return new R2(credentialsFor(bucket, env));
  }

  /** `key`'s bytes, or `undefined` when there is no such object. */
  async getObject(bucket: string, key: string): Promise<Uint8Array | undefined> {
    const answer = await this.#send("GET", bucket, key);
    if (answer.status === 404) return undefined;
    await expectOk(answer, `GET ${bucket}/${key}`);
    return new Uint8Array(await answer.arrayBuffer());
  }

  /** `key`'s size, ETag and type, or `undefined` when there is no such object. */
  async headObject(bucket: string, key: string): Promise<ObjectInfo | undefined> {
    const answer = await this.#send("HEAD", bucket, key);
    if (answer.status === 404) return undefined;
    await expectOk(answer, `HEAD ${bucket}/${key}`);
    return {
      size: Number(answer.headers.get("content-length") ?? 0),
      etag: answer.headers.get("etag") ?? undefined,
      contentType: answer.headers.get("content-type") ?? undefined,
    };
  }

  /** Stores `body` at `key`, served with `options`' headers. */
  async putObject(bucket: string, key: string, body: Body, options: PutOptions): Promise<void> {
    const { payload, length } = payloadOf(body);
    const headers: Record<string, string> = {
      "content-type": options.contentType,
      "content-length": String(length),
    };
    if (options.cacheControl !== undefined) headers["cache-control"] = options.cacheControl;
    await expectOk(await this.#send("PUT", bucket, key, headers, payload), `PUT ${bucket}/${key}`);
  }

  /** Removes `key`; removing an object that isn't there is no error. */
  async deleteObject(bucket: string, key: string): Promise<void> {
    await expectOk(await this.#send("DELETE", bucket, key), `DELETE ${bucket}/${key}`);
  }

  /**
   * Signs the method, address and headers, not the body (`UNSIGNED-PAYLOAD`, which R2 takes over
   * HTTPS), so a file streams without being read twice; then sends the body with them.
   */
  async #send(
    method: string,
    bucket: string,
    key: string,
    headers: Record<string, string> = {},
    payload?: BodyInit,
  ): Promise<Response> {
    const signed = await this.#aws.sign(objectUrl(this.#accountId, bucket, key), {
      method,
      headers: { ...headers, "x-amz-content-sha256": "UNSIGNED-PAYLOAD" },
    });
    const init: RequestInit & { duplex?: "half" } = { method, headers: signed.headers };
    if (payload !== undefined) {
      init.body = payload;
      // A stream body needs it in Node's fetch.
      init.duplex = "half";
    }
    return this.#fetch(signed.url, init);
  }
}

function payloadOf(body: Body): { payload: BodyInit; length: number } {
  if (typeof body === "string") {
    const bytes = new TextEncoder().encode(body);
    return { payload: bytes, length: bytes.length };
  }
  // A copy on its own ArrayBuffer, which fetch's body type takes.
  if (body instanceof Uint8Array) return { payload: new Uint8Array(body), length: body.length };
  const stream = Readable.toWeb(createReadStream(body.file)) as ReadableStream<Uint8Array>;
  return { payload: stream, length: statSync(body.file).size };
}

async function expectOk(answer: Response, what: string): Promise<void> {
  if (answer.ok) return;
  const detail = (await answer.text()).slice(0, 500);
  throw new Error(`${what} answered ${answer.status}: ${detail}`);
}

function option(argv: readonly string[], name: string): string | undefined {
  const at = argv.indexOf(name);
  return at >= 0 ? argv[at + 1] : undefined;
}

async function cli(argv: readonly string[]): Promise<void> {
  const [command, bucket, key, file] = argv;
  if (bucket === undefined || key === undefined) throw new Error("usage: r2 put|delete <bucket> <key> …");
  const r2 = R2.for(bucket);
  if (command === "put" && file !== undefined) {
    const cacheControl = option(argv, "--cache-control");
    await r2.putObject(
      bucket,
      key,
      { file },
      {
        contentType: option(argv, "--content-type") ?? "application/octet-stream",
        ...(cacheControl === undefined ? {} : { cacheControl }),
      },
    );
    console.log(`r2: put ${bucket}/${key}`);
  } else if (command === "delete") {
    await r2.deleteObject(bucket, key);
    console.log(`r2: deleted ${bucket}/${key}`);
  } else {
    throw new Error(
      "usage: r2 put <bucket> <key> <file> [--content-type …] [--cache-control …] | delete <bucket> <key>",
    );
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  cli(process.argv.slice(2)).catch((error: unknown) => {
    console.error(`r2: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  });
}
