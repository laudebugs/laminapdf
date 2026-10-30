// A Worker that generates a PDF per request: `/` a one-page letter for `?name=`, `/report` the
// 100-page report with two embedded fonts (examples/report.ts). The fonts are this repository's
// fixtures; in your own Worker, bundle your font files the same way (wrangler.toml `rules`).
import { Builder, Font, init } from "@lamina/pdf";
import notoSans from "../../../../../fixtures/fonts/NotoSans-Regular.ttf";
import notoSansSC from "../../../../../fixtures/fonts/NotoSansSC-Regular.otf";
import { report } from "../../report.js";

/** A one-page letter to `name`, in a standard 14 font (nothing embedded). */
function letter(name: string): Uint8Array {
  const b = new Builder({ producer: "lamina Worker example" });
  const page = b.page("A4");
  page.surface().text(`Hello, ${name}!`, { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
  page.finish();
  return b.finish().save();
}

const pdf = (bytes: Uint8Array): Response =>
  new Response(bytes as Uint8Array<ArrayBuffer>, { headers: { "content-type": "application/pdf" } });

export default {
  async fetch(request: Request): Promise<Response> {
    // Loads both modules once per isolate; later requests return at once.
    await init({ text: true });
    const url = new URL(request.url);
    switch (url.pathname) {
      case "/":
        return pdf(letter(url.searchParams.get("name") ?? "world"));
      case "/report":
        return pdf(report(new Uint8Array(notoSans), new Uint8Array(notoSansSC)));
      default:
        return new Response("not found", { status: 404 });
    }
  },
};
