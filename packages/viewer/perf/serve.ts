// The harness page's server: the built page and the fixtures, with the app's code and WebAssembly
// compressed as Cloudflare serves them (gzip here), so the cold first page downloads what a
// visitor downloads, and with a cache lifetime, so a reload is warm. PDFs go out as they are.

import { readFileSync, statSync } from "node:fs";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { extname, join, normalize, sep } from "node:path";
import { gzipSync } from "node:zlib";

const TYPES: Readonly<Record<string, string>> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".pdf": "application/pdf",
};
const COMPRESS = new Set([".html", ".js", ".css", ".wasm"]);

/** The file a request for `url` names under `base`: null for a malformed path, undefined for none. */
function fileFor(base: string, url: string | undefined): string | null | undefined {
  let path: string;
  try {
    path = decodeURIComponent(new URL(url ?? "/", "http://localhost").pathname);
  } catch {
    return null;
  }
  const file = normalize(join(base, path === "/" ? "index.html" : path));
  return file.startsWith(base) && statSync(file, { throwIfNoEntry: false })?.isFile() ? file : undefined;
}

/**
 * Answers a Range request for `bytes` as a file host does (the reader's linked files): one range
 * (`bytes=a-b`, `bytes=a-` or the last `bytes=-n`) with 206, else 416.
 */
function sendRange(response: ServerResponse, bytes: Uint8Array, headers: Record<string, string>, range: string): void {
  const size = bytes.length;
  const match = /^bytes=(\d*)-(\d*)$/.exec(range);
  const [, from = "", to = ""] = match ?? [];
  const start = Math.max(0, from === "" ? size - Number(to) : Number(from));
  const end = Math.min(size - 1, from === "" || to === "" ? size - 1 : Number(to));
  if (match === null || (from === "" && to === "") || start > end) {
    response.writeHead(416, { "Content-Range": `bytes */${size}` }).end();
    return;
  }
  response
    .writeHead(206, { ...headers, "Accept-Ranges": "bytes", "Content-Range": `bytes ${start}-${end}/${size}` })
    .end(bytes.subarray(start, end + 1));
}

/**
 * Sends `file`: compressed when its type is (as Cloudflare does) and the browser takes gzip, as a
 * range when a PDF is asked for one, else whole.
 */
function sendFile(request: IncomingMessage, response: ServerResponse, file: string, gzipped: Map<string, Uint8Array>) {
  const type = extname(file);
  const headers = { "Content-Type": TYPES[type] ?? "application/octet-stream", "Cache-Control": "max-age=3600" };
  if (COMPRESS.has(type) && /\bgzip\b/.test(request.headers["accept-encoding"] ?? "")) {
    let body = gzipped.get(file);
    if (body === undefined) {
      body = gzipSync(readFileSync(file));
      gzipped.set(file, body);
    }
    response.writeHead(200, { ...headers, "Content-Encoding": "gzip", Vary: "Accept-Encoding" }).end(body);
  } else if (type === ".pdf" && request.headers.range !== undefined) {
    sendRange(response, readFileSync(file), headers, request.headers.range);
  } else {
    response
      .writeHead(200, { ...headers, ...(type === ".pdf" ? { "Accept-Ranges": "bytes" } : {}) })
      .end(readFileSync(file));
  }
}

/** Serves `root` on a free local port until `close()`. */
export async function serve(root: string): Promise<{ readonly url: string; close(): Promise<void> }> {
  const base = normalize(root.endsWith(sep) ? root : `${root}${sep}`);
  const gzipped = new Map<string, Uint8Array>();
  const server = createServer((request, response) => {
    const file = fileFor(base, request.url);
    if (file === null || file === undefined) {
      response.writeHead(file === null ? 400 : 404).end();
      return;
    }
    sendFile(request, response, file, gzipped);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  return {
    url: `http://127.0.0.1:${port}/`,
    close: () => new Promise<void>((resolve) => server.close(() => resolve())),
  };
}
