// page.text() in a browser (B9c): the extract module through the `browser` export condition,
// fetched next to its glue, gives the shared snapshot's lines (fixtures/text/page-text.json,
// written from the Rust output), so the browser build extracts what Node and Rust extract.
import { Document, init } from "@laminapdf/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { server } from "vitest/browser";

const urls = import.meta.glob<string>(
  ["../../../../fixtures/golden/text-*.pdf", "../../../../fixtures/text/page-text.json"],
  { query: "?url", import: "default", eager: true },
);

async function fetchFixture(path: string): Promise<Response> {
  const url = urls[`../../../../fixtures/${path}`];
  if (url === undefined) throw new Error(`${path} is not among the fixtures this test serves`);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`fetching ${path}: ${response.status}`);
  return response;
}

beforeAll(() => init({ extract: true }));

describe(`page.text() in ${server.browser}`, () => {
  it("extracts every text golden as Rust does (five scripts included)", async () => {
    const snapshot = (await (await fetchFixture("text/page-text.json")).json()) as {
      files: Record<string, { lines: string[] }[]>;
    };
    const files = Object.keys(snapshot.files).filter((f) => f.startsWith("golden/text-"));
    expect(files.length).toBe(5);
    for (const file of files) {
      const bytes = new Uint8Array(await (await fetchFixture(file)).arrayBuffer());
      const doc = Document.open(bytes);
      const lines = doc.pages.map((_, i) => doc.page(i).text().lines);
      expect(lines, file).toEqual(snapshot.files[file]?.map((p) => p.lines));
    }
  });
});
