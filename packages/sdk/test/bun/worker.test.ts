// The worker client in a Bun web Worker (L2a): the bundled `new Worker(new URL(…))` path. Run
// with test/bun/examples.test.ts.
import { test } from "bun:test";
import { readFile } from "node:fs/promises";
import { workerSmoke } from "../suite/worker.js";

const root = new URL("../../../../", import.meta.url);

test(
  "the worker client opens and saves in a Bun worker",
  async () => workerSmoke(new Uint8Array(await readFile(new URL("fixtures/golden/examples/hello.pdf", root)))),
  60_000,
);
