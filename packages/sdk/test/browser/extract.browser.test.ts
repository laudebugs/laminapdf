// doc.extract() in a browser (L3e): the extract module through the `browser` export condition
// gives the JSON Rust writes (fixtures/extract/json/, the goldens crates/laminapdf/tests/extract.rs
// writes) and the Markdown its goldens hold.
import { Document, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { server } from "vitest/browser";

const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/reading-order/table-ordered.pdf",
    "../../../../fixtures/reading-order/two-columns-ordered.pdf",
    "../../../../fixtures/extract/json/*.json",
    "../../../../fixtures/extract/markdown/table-ordered.md",
  ],
  { query: "?url", import: "default", eager: true },
);

async function fetchFixture(path: string): Promise<Response> {
  const url = urls[`../../../../fixtures/${path}`];
  if (url === undefined) throw new Error(`${path} is not among the fixtures this test serves`);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`fetching ${path}: ${response.status}`);
  return response;
}

async function open(path: string): Promise<Document> {
  return Document.open(new Uint8Array(await (await fetchFixture(path)).arrayBuffer()));
}

beforeAll(() => init({ extract: true }));

describe(`doc.extract() in ${server.browser}`, () => {
  it("gives the JSON Rust gives", async () => {
    for (const stem of ["table-ordered", "two-columns-ordered"]) {
      const doc = await open(`reading-order/${stem}.pdf`);
      const json = doc.extract({ format: "json", pageMarkers: true, chunk: { maxTokens: 400 } });
      const golden = await (await fetchFixture(`extract/json/${stem}.json`)).json();
      expect({ ...json, generator: { ...json.generator, version: "*" } }, stem).toEqual(golden);
    }
  });

  it("gives the Markdown Rust gives", async () => {
    const doc = await open("reading-order/table-ordered.pdf");
    const golden = await (await fetchFixture("extract/markdown/table-ordered.md")).text();
    expect(doc.extract({ format: "markdown", pageMarkers: true })).toBe(golden);
  });
});
