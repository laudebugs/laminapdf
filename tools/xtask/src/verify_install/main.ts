// `cargo xtask verify-install` in Node, Deno and Bun: the quick start (hello.ts, copied from
// packages/sdk/examples/) against the installed tarballs, then the check line on stdout.
import { init } from "@laminapdf/sdk";
import { hello } from "./hello.ts";
import { HELLO_OPTIONS } from "./hello-options.ts";
import { report } from "./report.ts";

await init({ text: true });
console.log(await report(hello(HELLO_OPTIONS)));
