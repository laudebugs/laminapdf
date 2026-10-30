// The performance smoke in Node (pdf-library §4; recorded, not asserted) and the report's hash:
// its own file, so it runs in a fresh module instance (Vitest isolates files) before any other
// example. The `measure` project of vitest.node.config.ts, run by `cargo xtask pdf-runtimes
// node`; `cargo xtask wasm` leaves it out (its four 100-page reports).
import { versions } from "node:process";
import { it } from "vitest";
import { read } from "../helpers.js";
import { runSuite } from "../suite/examples.js";

runSuite(`node-${versions.node.split(".")[0]}`, read, it, "measure");
