// The shared example list (test/suite/examples.ts) inside workerd: `/example/<name>` answers
// with the example's bytes (the Node side compares them with the golden), `/measure` with the
// performance smoke's figures and the report's hash. Request `/measure` first, in a fresh
// isolate: hello-first is then cold and the WebAssembly memory is the hello and report runs'
// alone (memory never shrinks).
import { init } from "@laminapdf/sdk";
import invoiceTemplate from "../../../../fixtures/documents/invoice.json" with { type: "json" };
import invoiceData from "../../../../fixtures/documents/invoice-data.json" with { type: "json" };
import notoSans from "../../../../fixtures/fonts/NotoSans-Regular.ttf";
import notoSansCollection from "../../../../fixtures/fonts/NotoSans-RegularBold.ttc";
import notoSansArabic from "../../../../fixtures/fonts/NotoSansArabic-Regular.ttf";
import notoSansDevanagari from "../../../../fixtures/fonts/NotoSansDevanagari-Regular.ttf";
import notoSansHebrew from "../../../../fixtures/fonts/NotoSansHebrew-Regular.ttf";
import notoSansSC from "../../../../fixtures/fonts/NotoSansSC-Regular.otf";
import linksAndOutlinesPdf from "../../../../fixtures/golden/examples/links_and_outlines.pdf";
import pdfaArchive2b from "../../../../fixtures/golden/examples/pdf_a_archive-2b.pdf";
import metadataPdf from "../../../../fixtures/golden/metadata.pdf";
import photo from "../../../../fixtures/images/photo-rgb.jpg";
import report from "../../../../fixtures/markdown/report.md";
import formsPdf from "../../../../fixtures/pages/forms.pdf";
import labelsPdf from "../../../../fixtures/pages/labels.pdf";
import referencesPdf from "../../../../fixtures/pages/references.pdf";
import flowReport from "../../../../fixtures/text/element-flow-report.json" with { type: "json" };
import { EXAMPLES, INPUT_PATHS, type Inputs, measure, missingInputs, sha256 } from "../suite/examples.js";

// Workers can't read files: the inputs are bundled (wrangler.toml `rules`). Imports must be
// static, so this lists them again, keyed by the suite's paths; the check below fails the Worker
// at load when the suite's INPUT_PATHS name one that is missing here.
const inputs: Inputs = {
  ...Object.fromEntries(
    Object.entries({
      "fixtures/fonts/NotoSans-Regular.ttf": notoSans,
      "fixtures/fonts/NotoSansArabic-Regular.ttf": notoSansArabic,
      "fixtures/fonts/NotoSansDevanagari-Regular.ttf": notoSansDevanagari,
      "fixtures/fonts/NotoSansHebrew-Regular.ttf": notoSansHebrew,
      "fixtures/fonts/NotoSansSC-Regular.otf": notoSansSC,
      "fixtures/images/photo-rgb.jpg": photo,
      "fixtures/fonts/NotoSans-RegularBold.ttc": notoSansCollection,
      "fixtures/markdown/report.md": report,
      "fixtures/golden/metadata.pdf": metadataPdf,
      "fixtures/golden/examples/pdf_a_archive-2b.pdf": pdfaArchive2b,
      "fixtures/pages/references.pdf": referencesPdf,
      "fixtures/pages/forms.pdf": formsPdf,
      "fixtures/pages/labels.pdf": labelsPdf,
      "fixtures/golden/examples/links_and_outlines.pdf": linksAndOutlinesPdf,
    }).map(([path, data]) => [path, new Uint8Array(data)]),
  ),
  // The element-flow report comes in as a JSON module; the suite reads every input as bytes.
  "fixtures/text/element-flow-report.json": new TextEncoder().encode(JSON.stringify(flowReport)),
  "fixtures/documents/invoice.json": new TextEncoder().encode(JSON.stringify(invoiceTemplate)),
  "fixtures/documents/invoice-data.json": new TextEncoder().encode(JSON.stringify(invoiceData)),
};
const missing = missingInputs(inputs, INPUT_PATHS);
if (missing.length > 0) throw new Error(`test/workerd/worker.ts doesn't bundle ${missing.join(", ")}`);

// Workers' scheduler (https://developers.cloudflare.com/workers/runtime-apis/scheduler/).
declare const scheduler: { wait(ms: number): Promise<void> };

// workerd's clock only advances across I/O (a Spectre mitigation): without a wait between the
// two readings every sample would be 0 ms. A zero-length `scheduler.wait` is the cheapest I/O.
const settle = (): Promise<void> => scheduler.wait(0);

// A default export because that is how a Worker module exposes its handlers (the named-exports
// rule is for package code).
export default {
  async fetch(request: Request): Promise<Response> {
    await init({ text: true });
    const path = new URL(request.url).pathname;
    if (path === "/measure") {
      const { measurements, report } = await measure("workerd", inputs, settle);
      return Response.json({ measurements, reportSha256: await sha256(report) });
    }
    const example = EXAMPLES.find((e) => path === `/example/${e.name}`);
    if (example === undefined) return new Response("not found", { status: 404 });
    return new Response(example.build(inputs) as Uint8Array<ArrayBuffer>, {
      headers: { "content-type": "application/pdf" },
    });
  },
};
