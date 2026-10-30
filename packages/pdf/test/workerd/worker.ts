// The shared example list (test/suite/examples.ts) inside workerd: `/example/<name>` answers
// with the example's bytes (the Node side compares them with the golden), `/measure` with the
// performance smoke's figures and the report's hash. Request `/measure` first, in a fresh
// isolate: hello-first is then cold and the WebAssembly memory is the hello and report runs'
// alone (memory never shrinks).
import { init } from "@lamina/pdf";
import notoSans from "../../../../fixtures/fonts/NotoSans-Regular.ttf";
import notoSansArabic from "../../../../fixtures/fonts/NotoSansArabic-Regular.ttf";
import notoSansDevanagari from "../../../../fixtures/fonts/NotoSansDevanagari-Regular.ttf";
import notoSansHebrew from "../../../../fixtures/fonts/NotoSansHebrew-Regular.ttf";
import notoSansSC from "../../../../fixtures/fonts/NotoSansSC-Regular.otf";
import photo from "../../../../fixtures/images/photo-rgb.jpg";
import { EXAMPLES, type Inputs, measure, sha256 } from "../suite/examples.js";

// Workers can't read files: the inputs are bundled (wrangler.toml `rules`).
const inputs: Inputs = Object.fromEntries(
  Object.entries({
    "fixtures/fonts/NotoSans-Regular.ttf": notoSans,
    "fixtures/fonts/NotoSansArabic-Regular.ttf": notoSansArabic,
    "fixtures/fonts/NotoSansDevanagari-Regular.ttf": notoSansDevanagari,
    "fixtures/fonts/NotoSansHebrew-Regular.ttf": notoSansHebrew,
    "fixtures/fonts/NotoSansSC-Regular.otf": notoSansSC,
    "fixtures/images/photo-rgb.jpg": photo,
  }).map(([path, data]) => [path, new Uint8Array(data)]),
);

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
