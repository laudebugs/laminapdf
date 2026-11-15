// R2's objects through its S3-compatible API (SigV4, with aws4fetch), for the scripts that write
// the desktop releases' buckets: the manifest tool, the desktop update test, and the desktop
// release workflow. Not Wrangler's `r2 object` commands: they call Cloudflare's REST API, which an
// R2 token scoped to one bucket (Object Read & Write, apps/updates/README.md) can't use, and such
// a token is all the workflows hold.
//
// An R2 API token's S3 credentials: the Access Key ID shown with the token, and the SHA-256 of the
// token's value as the secret. Each bucket's token and key ID come from the environment
// (`credentialsFor`), so a job holds only the buckets its tokens name.
//
//   node scripts/r2.ts put <bucket> <key> <file> [--content-type <type>] [--cache-control <value>]
//   node scripts/r2.ts delete <bucket> <key>
import { createHash } from "node:crypto";
import { createReadStream, statSync } from "node:fs";
import process from "node:process";
import { Readable } from "node:stream";
import { setTimeout as sleep } from "node:timers/promises";
import { pathToFileURL } from "node:url";
import { AwsClient } from "aws4fetch";
import { CHANNEL_BUCKETS } from "../src/channels.ts";

/** An R2 token's S3 credentials, on one account. */
export interface R2Credentials {
  readonly accountId: string;
  readonly accessKeyId: string;
  /** The token's value; the S3 secret is its SHA-256. */
  readonly token: string;
}

type Env = Readonly<Record<string, string | undefined>>;

/**
 * The environment's names for `bucket`'s credentials: the staging bucket's are
 * CLOUDFLARE_STAGING_R2_TOKEN and STAGING_R2_ACCESS_KEY_ID (the release workflow holds it beside
 * the releases bucket's); every other bucket's are the job's CLOUDFLARE_API_TOKEN and
 * R2_ACCESS_KEY_ID, the environment's own, which override the repository's deploy token of that
 * name. CLOUDFLARE_ACCOUNT_ID is shared.
 */
function credentialNames(bucket: string): { readonly token: string; readonly accessKeyId: string } {
  return bucket === CHANNEL_BUCKETS.staging.bucket
    ? { token: "CLOUDFLARE_STAGING_R2_TOKEN", accessKeyId: "STAGING_R2_ACCESS_KEY_ID" }
    : { token: "CLOUDFLARE_API_TOKEN", accessKeyId: "R2_ACCESS_KEY_ID" };
}

/** `bucket`'s credentials from `env`; throws naming what is missing (names only, never values). */
export function credentialsFor(bucket: string, env: Env): R2Credentials {
  const names = credentialNames(bucket);
  const accountId = env.CLOUDFLARE_ACCOUNT_ID;
  const accessKeyId = env[names.accessKeyId];
  const token = env[names.token];
  if (accountId && accessKeyId && token) return { accountId, accessKeyId, token };
  const missing = [
    accountId ? undefined : "CLOUDFLARE_ACCOUNT_ID",
    accessKeyId ? undefined : names.accessKeyId,
    token ? undefined : names.token,
  ].filter((name) => name !== undefined);
  throw new Error(`${bucket} needs ${missing.join(", ")} in the environment`);
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

/** What to store: text, bytes, or a file read as a stream (an installer needn't sit in memory). */
export type ObjectBody = string | Uint8Array | { readonly file: string };

/** The headers a stored object is served with. */
export interface PutOptions {
  readonly contentType: string;
  readonly cacheControl?: string;
}

/** How often a request is sent when R2 answers 429 or 5xx, and the first wait between tries. */
const TRIES = 4;
const FIRST_WAIT_MS = 500;

/** One R2 bucket, through the S3 API, with its own token. */
export class R2Bucket {
  readonly name: string;
  readonly #aws: AwsClient;
  readonly #accountId: string;
  readonly #fetch: typeof fetch;
  readonly #wait: (ms: number) => Promise<unknown>;

  constructor(
    name: string,
    credentials: R2Credentials,
    fetcher: typeof fetch = fetch,
    wait: (ms: number) => Promise<unknown> = sleep,
  ) {
    this.name = name;
    this.#accountId = credentials.accountId;
    this.#fetch = fetcher;
    this.#wait = wait;
    this.#aws = new AwsClient({
      accessKeyId: credentials.accessKeyId,
      secretAccessKey: secretOf(credentials.token),
      service: "s3",
      region: "auto",
    });
  }

  /** The bucket `name` with its credentials from `env`. */
  static open(name: string, env: Env = process.env): R2Bucket {
    return new R2Bucket(name, credentialsFor(name, env));
  }

  /** `key`'s bytes, or `undefined` when the bucket has no such key (a missing bucket fails). */
  async getObject(key: string): Promise<Uint8Array | undefined> {
    const answer = await this.#send("GET", key);
    if (answer.status === 404) {
      const detail = await answer.text();
      if (/<Code>NoSuchKey<\/Code>/.test(detail)) return undefined;
      throw new Error(`GET ${this.name}/${key} answered 404: ${detail.slice(0, 500)}`);
    }
    await expectOk(answer, `GET ${this.name}/${key}`);
    return new Uint8Array(await answer.arrayBuffer());
  }

  /** `key`'s size, ETag and type, or `undefined` when there is no such object. */
  async headObject(key: string): Promise<ObjectInfo | undefined> {
    const answer = await this.#send("HEAD", key);
    if (answer.status === 404) return undefined;
    await expectOk(answer, `HEAD ${this.name}/${key}`);
    return {
      size: Number(answer.headers.get("content-length") ?? 0),
      etag: answer.headers.get("etag") ?? undefined,
      contentType: answer.headers.get("content-type") ?? undefined,
    };
  }

  /** Stores `body` at `key`, served with `options`' headers. */
  async putObject(key: string, body: ObjectBody, options: PutOptions): Promise<void> {
    const headers: Record<string, string> = { "content-type": options.contentType };
    if (options.cacheControl !== undefined) headers["cache-control"] = options.cacheControl;
    await expectOk(await this.#send("PUT", key, headers, body), `PUT ${this.name}/${key}`);
  }

  /** Removes `key`; removing an object that isn't there is no error. */
  async deleteObject(key: string): Promise<void> {
    await expectOk(await this.#send("DELETE", key), `DELETE ${this.name}/${key}`);
  }

  /**
   * Signs the method, address and headers, not the body (`UNSIGNED-PAYLOAD`, which R2 takes over
   * HTTPS), so a file streams without being read twice; then sends the body with them. A 429 or
   * 5xx is tried again, with a fresh body each time, after a doubling wait.
   */
  async #send(method: string, key: string, headers: Record<string, string> = {}, body?: ObjectBody): Promise<Response> {
    for (let attempt = 1; ; attempt++) {
      const payload = body === undefined ? undefined : payloadOf(body);
      const signed = await this.#aws.sign(objectUrl(this.#accountId, this.name, key), {
        method,
        headers: {
          ...headers,
          ...(payload === undefined ? {} : { "content-length": String(payload.length) }),
          "x-amz-content-sha256": "UNSIGNED-PAYLOAD",
        },
      });
      const init: RequestInit & { duplex?: "half" } = { method, headers: signed.headers };
      if (payload !== undefined) {
        init.body = payload.payload;
        // A stream body needs it in Node's fetch.
        init.duplex = "half";
      }
      const answer = await this.#fetch(signed.url, init);
      const retry = answer.status === 429 || answer.status >= 500;
      if (!retry || attempt === TRIES) return answer;
      await answer.body?.cancel();
      await this.#wait(FIRST_WAIT_MS * 2 ** (attempt - 1));
    }
  }
}

function payloadOf(body: ObjectBody): { payload: BodyInit; length: number } {
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

const USAGE = "usage: r2 put <bucket> <key> <file> [--content-type …] [--cache-control …] | delete <bucket> <key>";

async function cli(argv: readonly string[]): Promise<void> {
  const [command, name, key, file] = argv;
  if (name === undefined || key === undefined) throw new Error(USAGE);
  const bucket = R2Bucket.open(name);
  if (command === "put" && file !== undefined) {
    const cacheControl = option(argv, "--cache-control");
    await bucket.putObject(
      key,
      { file },
      {
        contentType: option(argv, "--content-type") ?? "application/octet-stream",
        ...(cacheControl === undefined ? {} : { cacheControl }),
      },
    );
    console.log(`r2: put ${name}/${key}`);
  } else if (command === "delete") {
    await bucket.deleteObject(key);
    console.log(`r2: deleted ${name}/${key}`);
  } else {
    throw new Error(USAGE);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  cli(process.argv.slice(2)).catch((error: unknown) => {
    console.error(`r2: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  });
}
