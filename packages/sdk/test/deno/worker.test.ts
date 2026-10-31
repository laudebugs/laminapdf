// The worker client in a Deno web Worker (L2a): the bundled `new Worker(new URL(…))` path, the
// worker loading its `.wasm` from a file: URL. Run with test/deno/examples.test.ts.
import { workerSmoke } from "../suite/worker.js";

const root = new URL("../../../../", import.meta.url);

Deno.test("the worker client opens and saves in a Deno worker", async () => {
  await workerSmoke(await Deno.readFile(new URL("fixtures/golden/examples/hello.pdf", root)));
});
