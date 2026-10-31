// `cargo xtask verify-install workerd`: a Worker over the installed tarballs (the `workerd` export
// condition: the `.wasm` files bundled as modules) that answers every request with the check line.
import { init } from "@laminapdf/pdf";
import { hello } from "./hello.ts";
import { report } from "./report.ts";

// A default export because that is how a Worker module exposes its handlers.
export default {
  async fetch(): Promise<Response> {
    await init({ text: true });
    return new Response(await report(hello()));
  },
};
