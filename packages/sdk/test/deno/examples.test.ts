// The shared example list (test/suite/examples.ts) in Deno, through the `deno` export condition
// (dist/index.js: the glue fetches its `.wasm` next to itself, a file: URL). Run by
// `cargo xtask pdf-runtimes deno`: `deno test --sloppy-imports --no-check --allow-read` (the
// suite imports `.ts` files by their `.js` paths, as tsc wants; tsc checks the types, with
// deno.d.ts standing in for Deno's). The measurement registers first, so it runs cold in this
// file's fresh module instance.
import { hello } from "../../examples/quick-start/deno.js";
import { registerQuickStart, runSuite } from "../suite/examples.js";

const root = new URL("../../../../", import.meta.url);

const read = (path: string) => Deno.readFile(new URL(path, root));
runSuite("deno", read, Deno.test);
registerQuickStart("deno", read, Deno.test, hello);
