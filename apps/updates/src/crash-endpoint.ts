// `POST /crash` (D7a): takes one scrubbed crash report from the desktop app's main process and
// stores it in its own private bucket, `crash-reports/{yyyy}/{mm}/{dd}/{report_id}.json`, which a
// lifecycle rule empties after 30 days (apps/updates/README.md). Nothing here logs a body, and the
// client's IP is the rate limiter's key and nothing else: never logged, stored or put in an error.
// handler.ts turns the outcome into the response; it also refuses a request carrying the install ID.
import { type CrashReport, checkCrashReport } from "./crash.ts";

/** The largest body taken, in bytes. */
export const MAX_CRASH_BYTES = 65_536;

/** Where accepted reports go: the `CRASH_REPORTS` R2 bucket binding satisfies it. */
export interface CrashStore {
  put(key: string, value: string, options: { httpMetadata: { contentType: string } }): Promise<unknown>;
}

/** The per-IP limit: the `CRASH_LIMIT` Workers Rate Limiting binding satisfies it. */
export interface CrashLimiter {
  limit(options: { key: string }): Promise<{ success: boolean }>;
}

/** What the crash endpoint logs: only a failed write, with R2's own error text. */
export interface CrashLogEvent {
  readonly event: "crash-store-failed";
  /** The R2 error's name and message, which never hold the report or the request. */
  readonly error: { readonly name: string; readonly message: string };
}

/** What {@link acceptCrash} needs from its runtime. */
export interface CrashDeps {
  /** The crash-reports bucket, apart from every release bucket. */
  readonly crashStore: CrashStore;
  /** 10 reports a minute per IP (wrangler.jsonc's `ratelimits`). */
  readonly crashLimit: CrashLimiter;
  /** The current time in milliseconds, for the key's UTC date. */
  readonly now: () => number;
  readonly log: (entry: CrashLogEvent) => void;
}

/** How a report was answered: 202 stored, or the refusal's status and a short reason. */
export type CrashOutcome =
  | { readonly status: 202 }
  | { readonly status: 400 | 413 | 415 | 429 | 503; readonly reason: string };

/** The report's R2 key, by the UTC day it arrived. */
export function crashKey(report: CrashReport, now: number): string {
  const [yyyy, mm, dd] = new Date(now).toISOString().slice(0, 10).split("-");
  return `crash-reports/${yyyy}/${mm}/${dd}/${report.report_id}.json`;
}

function isJson(contentType: string | null): boolean {
  return contentType?.split(";")[0]?.trim().toLowerCase() === "application/json";
}

/** The body's bytes, or null past {@link MAX_CRASH_BYTES}: at most one byte more is read. */
async function readCapped(body: ReadableStream<Uint8Array> | null): Promise<Uint8Array | null> {
  if (body === null) return new Uint8Array();
  const reader = body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > MAX_CRASH_BYTES) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(size);
  let at = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, at);
    at += chunk.byteLength;
  }
  return bytes;
}

/** The body as a checked report, or the refusal. */
async function readReport(request: Request): Promise<CrashReport | CrashOutcome> {
  if (!isJson(request.headers.get("Content-Type"))) return { status: 415, reason: "send application/json" };
  const declared = Number(request.headers.get("Content-Length") ?? 0);
  if (declared > MAX_CRASH_BYTES) return { status: 413, reason: "a report is at most 64 KiB" };
  const bytes = await readCapped(request.body);
  if (bytes === null) return { status: 413, reason: "a report is at most 64 KiB" };
  let value: unknown;
  try {
    value = JSON.parse(new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(bytes));
  } catch {
    return { status: 400, reason: "not JSON" };
  }
  const checked = checkCrashReport(value);
  // The validator's errors name a JSON path and a rule, never anything from the body.
  return checked.ok ? checked.report : { status: 400, reason: checked.errors[0] ?? "not a crash report" };
}

/** Takes one crash report: limits, reads, checks and stores it. */
export async function acceptCrash(request: Request, deps: CrashDeps): Promise<CrashOutcome> {
  const ip = request.headers.get("CF-Connecting-IP") ?? "";
  let allowed: boolean;
  try {
    allowed = (await deps.crashLimit.limit({ key: ip })).success;
  } catch {
    return { status: 503, reason: "try again later" };
  }
  if (!allowed) return { status: 429, reason: "too many reports" };
  const report = await readReport(request);
  if ("status" in report) return report;
  try {
    // The checked value, written again: only the fields the schema allows reach the bucket.
    await deps.crashStore.put(crashKey(report, deps.now()), JSON.stringify(report), {
      httpMetadata: { contentType: "application/json" },
    });
  } catch (cause) {
    const error = cause instanceof Error ? cause : new Error(String(cause));
    deps.log({ event: "crash-store-failed", error: { name: error.name, message: error.message } });
    return { status: 503, reason: "try again later" };
  }
  return { status: 202 };
}
