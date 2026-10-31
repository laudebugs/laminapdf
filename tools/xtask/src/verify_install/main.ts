// `cargo xtask verify-install` in Node, Deno and Bun: the quick start (hello.ts, copied from
// packages/pdf/examples/) against the installed tarballs, then the check line on stdout.
import { init } from "@laminapdf/pdf";
import { hello } from "./hello.ts";
import { report } from "./report.ts";

await init({ text: true });
console.log(await report(hello()));
