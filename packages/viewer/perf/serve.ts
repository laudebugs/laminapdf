// The harness page's server: the built page and the fixtures, with the app's code and WebAssembly
// compressed as Cloudflare serves them (gzip here), so the cold first page downloads what a
// visitor downloads, and with a cache lifetime, so a reload is warm. PDFs go out as they are.

import { readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
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
    const type = extname(file);
    const headers = { "Content-Type": TYPES[type] ?? "application/octet-stream", "Cache-Control": "max-age=3600" };
    if (COMPRESS.has(type) && /\bgzip\b/.test(request.headers["accept-encoding"] ?? "")) {
      let body = gzipped.get(file);
      if (body === undefined) {
        body = gzipSync(readFileSync(file));
        gzipped.set(file, body);
      }
      response.writeHead(200, { ...headers, "Content-Encoding": "gzip", Vary: "Accept-Encoding" }).end(body);
    } else {
      response.writeHead(200, headers).end(readFileSync(file));
    }
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  return {
    url: `http://127.0.0.1:${port}/`,
    close: () => new Promise<void>((resolve) => server.close(() => resolve())),
  };
}
