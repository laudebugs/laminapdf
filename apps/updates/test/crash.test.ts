// POST /crash (D7a): the report's validator held to schemas/crash-report.schema.json case by case,
// the refusals of path-like free text, and the endpoint through `handle` in Node: size cap, content
// type, rate limit, install-ID refusal, the R2 key and R2 failing. test/log-leak.test.ts checks
// that no body or IP reaches the console.
import { readFileSync } from "node:fs";
import { Ajv2020 } from "ajv/dist/2020.js";
import { beforeEach, describe, expect, it } from "vitest";
import { checkCrashReport, isPathLike, SCRIPT_FILE } from "../src/crash.ts";
import { MAX_CRASH_BYTES } from "../src/crash-endpoint.ts";
import { handle } from "../src/handler.ts";
import { crashFixture, crashRequest, type Harness, harness, nativeCrashFixture } from "./helpers.ts";
import { wranglerConfig } from "./wrangler-config.ts";

const schema: object = JSON.parse(
  readFileSync(new URL("../schemas/crash-report.schema.json", import.meta.url), "utf8"),
);
const validate = new Ajv2020({ allErrors: true }).compile(schema);

// The fixture as plain JSON the cases below may change.
type Loose = Record<string, unknown> & { frames: Record<string, unknown>[]; location: Record<string, unknown> };

function edited(edit: (r: Loose) => void, base: Record<string, unknown> = crashFixture()): Loose {
  const copy = structuredClone(base) as Loose;
  edit(copy);
  return copy;
}

/** A webview report: script frames from the app's bundle and one from outside it. */
function webviewReport(): Loose {
  return edited((r) => {
    r.process = "webview";
    r.kind = "error";
    r.reason = "TypeError: Cannot read properties of undefined (reading 'pages')";
    r.sandbox = [];
    r.location = null as unknown as Record<string, unknown>;
    r.frames = [
      { file: "assets/index-abc123.js", line: 1, column: 48_211, function: "renderPage" },
      { file: "assets/viewer.worker-9f8e7d.mjs", line: 2, column: 77, function: null },
      { file: "<external>", line: 0, column: 0, function: null },
    ];
  });
}

function errorsOf(value: unknown): string[] {
  const checked = checkCrashReport(value);
  return checked.ok ? [] : checked.errors;
}

describe("checkCrashReport", () => {
  it("accepts the fixture, a native crash's and a webview report, as the schema does", () => {
    for (const report of [crashFixture(), nativeCrashFixture(), webviewReport()]) {
      expect(errorsOf(report)).toEqual([]);
      expect(validate(report), JSON.stringify(validate.errors)).toBe(true);
    }
  });

  // Each case breaks one rule the schema states; both must refuse it.
  const broken: [string, (r: Loose) => void][] = [
    ["an unknown schema", (r) => (r.schema = 2)],
    ["an unknown key", (r) => (r.install_id = "0b7d4c1e-2f3a-4b5c-8d6e-7f8091a2b3c4")],
    ["a missing reason", (r) => delete r.reason],
    ["an upper-case report ID", (r) => (r.report_id = "3F1C2A9E-8B7D-4E6F-9A5B-1C2D3E4F5A6B")],
    ["a UUID that isn't v4", (r) => (r.report_id = "3f1c2a9e-8b7d-1e6f-9a5b-1c2d3e4f5a6b")],
    ["an unknown channel", (r) => (r.channel = "canary")],
    ["an unknown OS", (r) => (r.os = "darwin")],
    ["an os_version with a slash", (r) => (r.os_version = "15/1")],
    ["an unknown sandbox layer", (r) => (r.sandbox = ["chroot"])],
    ["a sandbox layer twice", (r) => (r.sandbox = ["seatbelt", "seatbelt"])],
    ["an unknown uptime", (r) => (r.uptime = "forever")],
    ["a reason over 1024 characters", (r) => (r.reason = "a".repeat(1025))],
    ["a location outside lamina's sources", (r) => (r.location.file = "src/main.rs")],
    ["a negative line", (r) => (r.location.line = -1)],
    ["a fractional column", (r) => (r.location.column = 1.5)],
    ["a frame with an upper-case offset", (r) => (r.frames[0] = { ...r.frames[0], offset: "0x1A" })],
    ["a frame with both a module and a file", (r) => (r.frames[0] = { ...r.frames[0], file: "<external>" })],
    ["a module name with a slash", (r) => (r.frames[0] = { ...r.frames[0], module: "lib/a.so" })],
    ["a function over 256 characters", (r) => (r.frames[0] = { ...r.frames[0], function: "f".repeat(257) })],
    ["65 frames", (r) => (r.frames = Array.from({ length: 65 }, () => ({ ...r.frames[1] })))],
    ["a module with a short debug ID", (r) => (r.modules = [{ name: "a.dylib", version: null, debug_id: "abc" }])],
    ["a script frame outside assets/", (r) => (r.frames = [{ file: "main.js", line: 1, column: 1, function: null }])],
    [
      "a file: URL as a frame's file",
      (r) => (r.frames = [{ file: "file:///Users/x/a.js", line: 1, column: 1, function: null }]),
    ],
  ];

  // A native crash's stack (D7d), one rule broken at a time.
  type Native = { crash_reason: string; frames: Record<string, unknown>[]; modules: Record<string, unknown>[] };
  const native = (edit: (n: Native) => void) => (r: Loose) => {
    edit((r as unknown as { native: Native }).native);
  };
  const brokenNative: [string, (r: Loose) => void][] = [
    ["a native stack with an unknown key", (r) => (r.native = { ...(r.native as object), raw: "AAAA" })],
    ["a native stack without frames", native((n) => delete (n as Partial<Native>).frames)],
    ["a native offset as hex text", native((n) => (n.frames[0] = { ...n.frames[0], offset: "0x1a" }))],
    ["a negative native offset", native((n) => (n.frames[0] = { ...n.frames[0], offset: -1 }))],
    ["a native offset past 2^53", native((n) => (n.frames[0] = { ...n.frames[0], offset: 2 ** 53 }))],
    ["a native module with a path", native((n) => (n.frames[0] = { ...n.frames[0], module: "/usr/lib/a.so" }))],
    ["a native module without a debug ID", native((n) => (n.modules[0] = { ...n.modules[0], debug_id: null }))],
    ["a native crash reason over 128 characters", native((n) => (n.crash_reason = "a".repeat(129)))],
    ["a native frame function that isn't null", native((n) => (n.frames[0] = { ...n.frames[0], function: "render" }))],
    ["65 native frames", native((n) => (n.frames = Array.from({ length: 65 }, () => ({ ...n.frames[0] }))))],
  ];

  it.each(brokenNative)("refuses %s", (_, edit) => {
    const value = edited(edit, nativeCrashFixture());
    expect(errorsOf(value)).not.toEqual([]);
    expect(validate(value)).toBe(false);
  });

  it.each(broken)("refuses %s", (_, edit) => {
    const value = edited(edit);
    expect(errorsOf(value)).not.toEqual([]);
    expect(validate(value)).toBe(false);
  });

  // Beyond the schema: free text that still holds something the app's scrubber removes.
  const pathLike = [
    "failed at /Users/alice/Documents/a.pdf",
    "in /home/a/b",
    "C:\\Users\\bob\\x.pdf",
    "c:/temp/x",
    "\\\\server\\share\\x.pdf",
    "~/Documents",
    "~\\Documents",
    "see file:report.pdf",
    "mail alice@example.com",
    "call me at 5551234567",
    "id 123456",
  ];

  it.each(pathLike)("refuses %j in the reason", (text) => {
    expect(errorsOf(edited((r) => (r.reason = text)))).toEqual([
      "/reason: holds a path, an e-mail address or a long number",
    ]);
  });

  it.each(pathLike)("refuses %j in a native frame's function", (text) => {
    const value = edited((r) => (r.frames[0] = { ...r.frames[0], function: text }));
    expect(errorsOf(value)).toEqual(["/frames/0/function: holds a path, an e-mail address or a long number"]);
  });

  it.each(pathLike)("refuses %j in a native stack's crash reason", (text) => {
    const value = edited((r) => {
      (r.native as { crash_reason: string }).crash_reason = text;
    }, nativeCrashFixture());
    expect(errorsOf(value)).toEqual(["/native/crash_reason: holds a path, an e-mail address or a long number"]);
  });

  it.each(pathLike)("refuses %j in a script frame's function", (text) => {
    const value = edited((r) => (r.frames[1] = { ...r.frames[1], function: text }), webviewReport());
    expect(errorsOf(value)).toEqual(["/frames/1/function: holds a path, an e-mail address or a long number"]);
  });

  it.each([
    "crash: exit code 0xc0000005",
    "signal 11 at 0x1a2b3c",
    "index out of bounds: the len is 3 but the index is 7 (in <path>)",
    "laminapdf_desktop_engine::worker::render",
    "<impl core::fmt::Debug for alloc::vec::Vec<u8>>::fmt",
    "and/or",
  ])("lets %j through", (text) => {
    expect(isPathLike(text)).toBe(false);
  });

  it("never puts a value from the body in an error", () => {
    const marker = "MARKER-a1b2c3";
    const value = edited((r) => {
      r[marker] = marker;
      r.channel = marker;
      r.reason = `/${marker}/x`;
    });
    expect(errorsOf(value).join("\n")).not.toContain(marker);
  });
});

let h: Harness;

beforeEach(() => {
  h = harness();
});

async function post(body: BodyInit | null, headers?: Record<string, string>): Promise<Response> {
  return handle(crashRequest(body, headers), h.deps);
}

/** A body that arrives in 4 KiB chunks with no Content-Length. */
function streamOf(bytes: number): ReadableStream<Uint8Array> {
  let sent = 0;
  return new ReadableStream({
    pull(controller) {
      if (sent >= bytes) return controller.close();
      const size = Math.min(4096, bytes - sent);
      controller.enqueue(new Uint8Array(size).fill(0x20));
      sent += size;
    },
  });
}

// The desktop's scrubber and this check, held to one list of cases (the Rust side reads it too):
// each input the scrubber changes is refused as it was, and every scrubbed result is accepted.
describe("the shared scrub cases", () => {
  const { cases } = JSON.parse(readFileSync(new URL("./fixtures/scrub-cases.json", import.meta.url), "utf8")) as {
    cases: { input: string; scrubbed: string }[];
  };

  it.each(cases.filter((c) => c.input !== c.scrubbed))("refuses $input as it was", ({ input }) => {
    expect(isPathLike(input)).toBe(true);
  });

  it.each(cases)("accepts $scrubbed once scrubbed", ({ scrubbed }) => {
    expect(isPathLike(scrubbed)).toBe(false);
  });
});

/** A field the schema closes to a list of values. */
interface Closed {
  readonly enum: readonly string[];
}

/** The parts of the schema the tests below read. */
interface SchemaShape {
  readonly properties: Record<"channel" | "os" | "arch" | "process" | "renderer" | "uptime" | "kind", Closed> & {
    readonly sandbox: { readonly items: Closed };
  };
  readonly $defs: {
    readonly script_frame: {
      readonly properties: { readonly file: { readonly oneOf: readonly { readonly pattern: string }[] } };
    };
  };
}

describe("the schema's closed fields", () => {
  const shape = schema as SchemaShape;
  const closed = [
    ["channel", shape.properties.channel.enum],
    ["os", shape.properties.os.enum],
    ["arch", shape.properties.arch.enum],
    ["process", shape.properties.process.enum],
    ["renderer", shape.properties.renderer.enum],
    ["uptime", shape.properties.uptime.enum],
    ["kind", shape.properties.kind.enum],
  ] as const;

  it.each(closed)("lets %s take each of the schema's values", (field, values) => {
    for (const value of values) expect(checkCrashReport({ ...crashFixture(), [field]: value }).ok).toBe(true);
  });

  it("lets the sandbox list each of the schema's layers", () => {
    for (const layer of shape.properties.sandbox.items.enum) {
      expect(checkCrashReport({ ...crashFixture(), sandbox: [layer] }).ok).toBe(true);
    }
  });

  it("takes the schema's pattern for a script frame's file", () => {
    expect(SCRIPT_FILE.source.replaceAll("\\/", "/")).toBe(shape.$defs.script_frame.properties.file.oneOf[0]?.pattern);
  });
});

describe("POST /crash", () => {
  it("stores the report under its UTC day and answers 202 with no body", async () => {
    const response = await post(JSON.stringify(crashFixture()));
    expect(response.status).toBe(202);
    expect(await response.text()).toBe("");
    expect(response.headers.get("Cache-Control")).toBe("no-store");
    expect(response.headers.get("Access-Control-Allow-Origin")).toBeNull();
    expect([...h.crashes.objects.keys()]).toEqual([
      "crash-reports/2027/01/20/3f1c2a9e-8b7d-4e6f-9a5b-1c2d3e4f5a6b.json",
    ]);
    const stored = h.crashes.objects.get("crash-reports/2027/01/20/3f1c2a9e-8b7d-4e6f-9a5b-1c2d3e4f5a6b.json");
    expect(stored?.contentType).toBe("application/json");
    expect(JSON.parse(stored?.text ?? "")).toEqual(crashFixture());
    expect(h.logs).toEqual([]);
  });

  it("writes the checked report again, not the bytes sent", async () => {
    const spaced = JSON.stringify(crashFixture(), null, 4);
    expect((await post(spaced, { "Content-Type": "application/json; charset=utf-8" })).status).toBe(202);
    const [stored] = h.crashes.objects.values();
    expect(stored?.text).toBe(JSON.stringify(crashFixture()));
  });

  it("dates the key by the clock in UTC", async () => {
    h.advance(12 * 60 * 60 * 1000);
    await post(JSON.stringify(crashFixture()));
    expect([...h.crashes.objects.keys()][0]).toMatch(/^crash-reports\/2027\/01\/21\//);
  });

  it("accepts a webview report with bundle frames and refuses one with a file: frame", async () => {
    expect((await post(JSON.stringify(webviewReport()))).status).toBe(202);
    const local = webviewReport();
    local.frames[0] = { file: "file:///Users/x/a.js", line: 1, column: 1, function: null };
    const response = await post(JSON.stringify(local));
    expect(response.status).toBe(400);
    expect(await response.text()).toBe("/frames/0/file: must be assets/{name}.js or <external>\n");
    expect(h.crashes.objects.size).toBe(1);
  });

  it.each([
    ["the reason", (r: Loose) => (r.reason = "open /Users/alice/secret.pdf failed")],
    ["a frame's function", (r: Loose) => (r.frames[0] = { ...r.frames[0], function: "load ~/secret.pdf" })],
  ])("refuses a path in %s with a reason that doesn't echo it", async (_, edit) => {
    const response = await post(JSON.stringify(edited(edit)));
    expect(response.status).toBe(400);
    const text = await response.text();
    expect(text).toMatch(/holds a path, an e-mail address or a long number\n$/);
    expect(text).not.toContain("secret");
    expect(h.crashes.objects.size).toBe(0);
  });

  it("answers 400 to a body that isn't JSON", async () => {
    const response = await post('{"reason": "/Users/alice');
    expect(response.status).toBe(400);
    expect(await response.text()).toBe("not JSON\n");
  });

  it("answers 400 to bytes that aren't UTF-8", async () => {
    expect((await post(new Uint8Array([0x7b, 0xff, 0x7d]))).status).toBe(400);
  });

  it("answers 415 to another content type", async () => {
    for (const type of ["text/plain", "application/x-www-form-urlencoded", "application/jsonp"]) {
      expect((await post(JSON.stringify(crashFixture()), { "Content-Type": type })).status).toBe(415);
    }
  });

  it("answers 413 when Content-Length is over 64 KiB, whatever the body", async () => {
    const response = await post(JSON.stringify(crashFixture()), { "Content-Length": String(MAX_CRASH_BYTES + 1) });
    expect(response.status).toBe(413);
    expect(h.crashes.objects.size).toBe(0);
  });

  it("answers 413 to a streamed body over 64 KiB, whatever the header says", async () => {
    expect((await post(streamOf(MAX_CRASH_BYTES + 1))).status).toBe(413);
    expect((await post(streamOf(10 * MAX_CRASH_BYTES))).status).toBe(413);
  });

  it("reads a body of exactly 64 KiB", async () => {
    const text = JSON.stringify(crashFixture());
    const padded = text + " ".repeat(MAX_CRASH_BYTES - text.length);
    expect((await post(padded)).status).toBe(202);
  });

  it("answers 429 after 10 reports from one IP, and limits each IP apart", async () => {
    const body = JSON.stringify(crashFixture());
    for (let i = 0; i < 10; i += 1) expect((await post(body)).status).toBe(202);
    const refused = await post(body);
    expect(refused.status).toBe(429);
    expect(await refused.text()).not.toContain("192.0.2.1");
    expect((await post(body, { "CF-Connecting-IP": "198.51.100.7" })).status).toBe(202);
    expect([...h.limiter.calls.keys()]).toEqual(["192.0.2.1", "198.51.100.7"]);
  });

  it("refuses a request carrying the install ID before anything else", async () => {
    const response = await post(JSON.stringify(crashFixture()), {
      "Lamina-Install-Id": "0b7d4c1e-2f3a-4b5c-8d6e-7f8091a2b3c4",
    });
    expect(response.status).toBe(400);
    expect(await response.text()).not.toContain("0b7d4c1e");
    expect(h.limiter.calls.size).toBe(0);
    expect(h.crashes.objects.size).toBe(0);
  });

  it("answers 405 with Allow: POST to other methods", async () => {
    for (const method of ["GET", "HEAD", "PUT"]) {
      const response = await handle(new Request("https://updates.laminapdf.com/crash", { method }), h.deps);
      expect(response.status).toBe(405);
      expect(response.headers.get("Allow")).toBe("POST");
    }
  });

  it("keeps GET, HEAD as the other routes' Allow", async () => {
    const response = await handle(new Request("https://updates.laminapdf.com/healthz", { method: "POST" }), h.deps);
    expect(response.status).toBe(405);
    expect(response.headers.get("Allow")).toBe("GET, HEAD");
  });

  it("answers 503 when R2 fails, logging only R2's error", async () => {
    h.deps.crashStore.put = async () => {
      throw new Error("R2 unavailable");
    };
    expect((await post(JSON.stringify(crashFixture()))).status).toBe(503);
    expect(h.logs).toEqual([{ event: "crash-store-failed", error: { name: "Error", message: "R2 unavailable" } }]);
  });
});

describe("wrangler.jsonc", () => {
  const config = wranglerConfig();
  const crashBucket = (buckets: readonly { binding: string; bucket_name: string }[] | undefined) =>
    buckets?.find((b) => b.binding === "CRASH_REPORTS")?.bucket_name;

  it("binds a crash-reports bucket of its own to each Worker", () => {
    expect(crashBucket(config.r2_buckets)).toBe("lamina-pdf-crash-reports");
    expect(crashBucket(config.env.staging?.r2_buckets)).toBe("lamina-pdf-crash-reports-staging");
  });

  it("limits each Worker to 10 reports a minute, on separate counters", () => {
    const limits = [config.ratelimits, config.env.staging?.ratelimits ?? []].map((list) =>
      list.find((l) => l.name === "CRASH_LIMIT"),
    );
    for (const limit of limits) expect(limit?.simple).toEqual({ limit: 10, period: 60 });
    expect(limits[0]?.namespace_id).not.toBe(limits[1]?.namespace_id);
  });
});
