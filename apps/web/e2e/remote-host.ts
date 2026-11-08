// The e2e suites' "remote host": the server a `#url=` link points to, on its own port, so it is
// another origin than the reader's. HTTPS, since the reader's policy upgrades every request (see
// @laminapdf/test-utils/wrangler-dev), with a certificate made by openssl for each run, which the browser context's
// ignoreHTTPSErrors accepts. Files are served by path:
//
//   /cors/ranges/<file>  CORS headers and Range support (206 answers, ETag)
//   /cors/whole/<file>   CORS headers, Range ignored (always 200 with the whole file)
//   /plain/<file>        no CORS headers: the browser refuses to let the reader see the answer
//
// Every request it receives is logged with its method, path, headers and the bytes sent, so a
// test can check what reached the host (the Referer, cookies, how much of the file was read).

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { createServer } from "node:https";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** One request the host received. */
export interface HostRequest {
  readonly method: string;
  readonly path: string;
  readonly headers: Readonly<Record<string, string | string[] | undefined>>;
  /** Body bytes sent in the answer. */
  readonly sent: number;
}

/** A running remote host. */
export interface RemoteHost {
  /** `https://127.0.0.1:<port>`. */
  readonly origin: string;
  /** Requests so far. */
  readonly requests: HostRequest[];
  /** Serves `bytes` as `name` under every route. */
  add(name: string, bytes: Uint8Array): void;
  close(): Promise<void>;
}

/** A self-signed certificate for 127.0.0.1, valid for a day. */
function certificate(): { key: Buffer; cert: Buffer } {
  const dir = mkdtempSync(join(tmpdir(), "lamina-remote-host-"));
  try {
    const key = join(dir, "key.pem");
    const cert = join(dir, "cert.pem");
    execFileSync(
      "openssl",
      [
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "1",
        "-subj",
        "/CN=127.0.0.1",
        "-keyout",
        key,
        "-out",
        cert,
      ],
      { stdio: "ignore" },
    );
    return { key: readFileSync(key), cert: readFileSync(cert) };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "Range, If-Range",
  "Access-Control-Expose-Headers": "Content-Range, Content-Length, ETag, Accept-Ranges",
  "Access-Control-Max-Age": "600",
};

/** `Range: bytes=a-b` or `bytes=-n` against a file of `size` bytes: [start, end] inclusive. */
function rangeOf(header: string | undefined, size: number): [number, number] | undefined {
  const m = /^bytes=(\d*)-(\d*)$/.exec(header ?? "");
  if (m === null || (m[1] === "" && m[2] === "")) return undefined;
  if (m[1] === "") return [Math.max(0, size - Number(m[2])), size - 1];
  return [Number(m[1]), m[2] === "" ? size - 1 : Math.min(size - 1, Number(m[2]))];
}

/** What a path asks for: the route's flags and the file's name. */
interface Route {
  readonly cors: boolean;
  readonly ranges: boolean;
  readonly name: string;
}

/** `/cors/ranges/<file>`, `/cors/whole/<file>` or `/plain/<file>`; undefined for anything else. */
function routeOf(path: string): Route | undefined {
  const [, route, kind, ...rest] = path.split("/");
  if (route === "plain" && kind !== undefined) return { cors: false, ranges: false, name: [kind, ...rest].join("/") };
  if (route === "cors" && (kind === "ranges" || kind === "whole")) {
    return { cors: true, ranges: kind === "ranges", name: rest.join("/") };
  }
  return undefined;
}

/** Answers with `body`, or the part `range` asks for; returns the bytes sent. */
function serve(res: ServerResponse, body: Uint8Array, route: Route, range: string | undefined): number {
  const headers: Record<string, string> = {
    ...(route.cors ? CORS : {}),
    "Content-Type": "application/pdf",
    "Cache-Control": "no-store",
    ETag: `"${body.length}"`,
  };
  const part = route.ranges ? rangeOf(range, body.length) : undefined;
  if (part === undefined) {
    res.writeHead(200, { ...headers, "Content-Length": String(body.length) });
    res.end(body);
    return body.length;
  }
  const [start, end] = part;
  res.writeHead(206, {
    ...headers,
    "Accept-Ranges": "bytes",
    "Content-Range": `bytes ${start}-${end}/${body.length}`,
    "Content-Length": String(end - start + 1),
  });
  res.end(body.subarray(start, end + 1));
  return end - start + 1;
}

/** Starts the host on a free port of 127.0.0.1. */
export async function startRemoteHost(): Promise<RemoteHost> {
  const files = new Map<string, Uint8Array>();
  const requests: HostRequest[] = [];
  const handle = (req: IncomingMessage, res: ServerResponse) => {
    const path = new URL(req.url ?? "/", "https://host").pathname;
    const route = routeOf(path);
    const body = route === undefined ? undefined : files.get(route.name);
    let sent = 0;
    if (req.method === "OPTIONS" || route === undefined || body === undefined) {
      res.writeHead(req.method === "OPTIONS" ? 204 : 404, route?.cors ? CORS : {});
      res.end();
    } else {
      sent = serve(res, body, route, req.headers.range);
    }
    requests.push({ method: req.method ?? "", path, headers: req.headers, sent });
  };
  const server = createServer(certificate(), handle);
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  return {
    origin: `https://127.0.0.1:${port}`,
    requests,
    add: (name, bytes) => files.set(name, bytes),
    close: () =>
      new Promise((resolve) => {
        server.closeAllConnections();
        server.close(() => resolve());
      }),
  };
}

/**
 * An 8-page PDF of about `megabytes` MB: six small pages, then two that each carry half the size
 * as content. Reading the first pages (and the viewer's prefetch of the next ones) needs only the
 * start and end of the file.
 */
export function largePdf(megabytes: number): Uint8Array {
  const half = Math.floor((megabytes * 1024 * 1024) / 2);
  const line = "0.5 g 10 10 20 20 re f\n";
  const big = line.repeat(Math.ceil(half / line.length));
  const stream = (content: string) => `<< /Length ${content.length} >>\nstream\n${content}endstream`;
  const small = 6;
  const pages = small + 2;
  // 1 catalog, 2 page tree, 3 font, then each page and its content.
  const objects = new Map<number, string>([
    [1, "<< /Type /Catalog /Pages 2 0 R >>"],
    [
      2,
      `<< /Type /Pages /Kids [${Array.from({ length: pages }, (_, i) => `${4 + 2 * i} 0 R`).join(" ")}] /Count ${pages} >>`,
    ],
    [3, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"],
  ]);
  for (let i = 0; i < pages; i++) {
    const content = i < small ? `BT /F1 24 Tf 72 700 Td (Page ${i + 1}) Tj ET\n` : big;
    objects.set(
      4 + 2 * i,
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents ${5 + 2 * i} 0 R /Resources << /Font << /F1 3 0 R >> >> >>`,
    );
    objects.set(5 + 2 * i, stream(content));
  }
  let text = "%PDF-1.7\n%\u00e2\u00e3\u00cf\u00d3\n";
  const offsets = new Map<number, number>();
  for (const [number, body] of objects) {
    offsets.set(number, Buffer.byteLength(text, "latin1"));
    text += `${number} 0 obj\n${body}\nendobj\n`;
  }
  const size = objects.size + 1;
  const xref = Buffer.byteLength(text, "latin1");
  text += `xref\n0 ${size}\n0000000000 65535 f \n`;
  for (let number = 1; number < size; number++) {
    text += `${String(offsets.get(number)).padStart(10, "0")} 00000 n \n`;
  }
  text += `trailer\n<< /Size ${size} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new Uint8Array(Buffer.from(text, "latin1"));
}
