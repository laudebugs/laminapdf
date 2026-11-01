// The shared example list (test/suite/examples.ts) in Bun, through the `node` export condition
// (Bun resolves it like Node: the `.wasm` read from @laminapdf/wasm). Run by
// `cargo xtask pdf-runtimes bun` (`bun test ./test/bun/`); bun.d.ts declares `bun:test` for tsc.
// The measurement registers first, so it runs cold in this file's fresh module instance.
import { test } from "bun:test";
import { readFile } from "node:fs/promises";
import { hello } from "../../examples/quick-start/bun.js";
import { registerQuickStart, runSuite } from "../suite/examples.js";

const root = new URL("../../../../", import.meta.url);

// Bun's default per-test timeout is 5 s; the report's four 100-page builds take longer on a busy
// machine. 60 s, as the Vitest configs.
const read = async (path: string) => new Uint8Array(await readFile(new URL(path, root)));
const register = (name: string, body: () => Promise<void>) => test(name, body, 60_000);
runSuite("bun", read, register);
registerQuickStart("bun", read, register, hello);
