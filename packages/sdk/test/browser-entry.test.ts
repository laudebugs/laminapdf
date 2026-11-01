// The `browser`/`default` entry (dist/index.js) in Node, with the modules served over HTTP the
// way a page fetches them: init(URL) and loadText(Response).
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Builder, Document, Font, init, isTextLoaded, loadText } from "../dist/index.js";
import { read } from "./helpers.js";

let server: Server;
let origin = "";

beforeAll(async () => {
  server = createServer((req, res) => {
    const name = (req.url ?? "").slice(1);
    const plain = name.startsWith("plain/");
    res.setHeader("content-type", plain ? "application/octet-stream" : "application/wasm");
    res.end(read(`packages/wasm/dist/${plain ? name.slice(6) : name}`));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});

afterAll(() => new Promise<void>((resolve) => server.close(() => resolve())));

describe("browser entry", () => {
  it("compiles the base module streaming from a URL", async () => {
    await init(new URL("/lamina_pdf_base.wasm", origin));
    expect(Document.open(read("fixtures/golden/examples/hello.pdf")).pageCount).toBe(1);
  });

  it("loads the text module from a Response without the wasm content type", async () => {
    await loadText(fetch(`${origin}/plain/lamina_pdf_text.wasm`));
    const b = new Builder();
    const page = b.page("A6");
    page.surface().text("Hi", { font: Font.standard("Helvetica"), size: 9, at: [9, 20] });
    page.finish();
    expect([isTextLoaded(), b.finish().pageCount]).toEqual([true, 1]);
  });
});
