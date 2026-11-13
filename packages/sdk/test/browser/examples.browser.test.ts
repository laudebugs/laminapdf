// The shared example list (test/suite/examples.ts) in a browser (Chromium, Firefox, WebKit:
// vitest.browser.config.ts), through the `browser` export condition. The fixtures are fetched
// as Vite assets (`?url` imports), not read with a Node API. The measurement registers first, so
// it runs cold in this file's fresh module instance.
import { describe, it } from "vitest";
import { server } from "vitest/browser";
import { hello } from "../../examples/quick-start/browser.js";
import { registerQuickStart, runSuite } from "../suite/examples.js";

// Every fixture the suite may read, as the URL the Vite server serves it at.
const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/golden/examples/*.pdf",
    "../../../../fixtures/golden/metadata.pdf",
    "../../../../fixtures/fonts/*.{ttf,otf,ttc}",
    "../../../../fixtures/images/photo-{rgb,gray}.jpg",
    "../../../../fixtures/markdown/report.md",
    "../../../../fixtures/text/element-flow-report.json",
    "../../../../fixtures/documents/invoice{,-data}.json",
  ],
  { query: "?url", import: "default", eager: true },
);

async function read(path: string): Promise<Uint8Array> {
  const url = urls[`../../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not among the fixtures this test serves`);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`fetching ${path}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

describe(`examples in ${server.browser}`, () => {
  runSuite(server.browser, read, it);
  registerQuickStart(server.browser, read, it, hello);
});
