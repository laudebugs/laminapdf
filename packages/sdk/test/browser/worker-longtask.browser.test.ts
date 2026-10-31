// L2a's long-task check: opening and fully rewriting a ~50 MB file through the worker client keeps
// the main thread free (opening alone is lazy and fast). The same work on the main thread is recorded beside it, as the baseline that shows the
// observer sees long tasks at all. Chromium only (the `longtask` entry type). Recorded as a measurement in the job summary
// (`cargo xtask pdf-runtimes chromium`), never asserted: no wall-clock assertions in the PR
// lanes (ADR 0018).
import { Builder, Document, Image, init } from "@laminapdf/sdk";
import { connect } from "@laminapdf/sdk/worker";
import { expect, it } from "vitest";
import { server } from "vitest/browser";
import { printMeasurement } from "../suite/examples.js";

/** A ~50 MB PDF: one page with an incompressible 3,600 × 3,500 RGBA noise image. */
function largePdf(): Uint8Array {
  const [w, h] = [3600, 3500];
  const pixels = new Uint8Array(w * h * 4);
  let x = 0x2545f491;
  for (let i = 0; i < pixels.length; i++) {
    x ^= x << 13;
    x ^= x >>> 17;
    x ^= x << 5;
    pixels[i] = x & 0xff;
  }
  const b = new Builder({
    id: new TextEncoder().encode("lamina-l2a-50-mb"),
    producer: "lamina tests",
    creationDate: "D:20261031120000Z",
  });
  const page = b.page("A4");
  page.surface().image(Image.rgba(w, h, pixels), [0, 0, 595, 842]);
  page.finish();
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

/** The longest main-thread task (ms) while `run` runs; 0 if none passed the 50 ms threshold. */
async function longestTask(run: () => Promise<void>): Promise<number> {
  const tasks: number[] = [];
  const observer = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) tasks.push(entry.duration);
  });
  observer.observe({ type: "longtask" });
  await run();
  // Let the observer deliver the last entries.
  await new Promise((resolve) => setTimeout(resolve, 100));
  observer.disconnect();
  return Math.max(0, ...tasks);
}

it.runIf(server.browser === "chromium")(
  "opening and rewriting a 50 MB file through the worker leaves the main thread free",
  async () => {
    await init();
    const bytes = largePdf();
    expect(bytes.byteLength).toBeGreaterThan(45 * 1024 * 1024);
    const client = await connect();
    const sync = await longestTask(async () => {
      const doc = Document.open(bytes);
      expect(doc.save({ mode: "fullRewrite" }).byteLength).toBeGreaterThan(0);
      doc.free();
    });
    const worker = await longestTask(async () => {
      await using doc = await client.open(bytes, { transfer: true });
      expect((await doc.save({ mode: "fullRewrite" })).byteLength).toBeGreaterThan(0);
    });
    client.terminate();
    printMeasurement({ runtime: "chromium", case: "sync-rewrite-50mb-longest-task", value: sync });
    printMeasurement({ runtime: "chromium", case: "worker-rewrite-50mb-longest-task", value: worker });
  },
);
