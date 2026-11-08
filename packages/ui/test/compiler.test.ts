import { readdirSync, readFileSync } from "node:fs";
import { transformAsync } from "@babel/core";
import reactCompiler from "babel-plugin-react-compiler";
import { describe, expect, it } from "vitest";

/** Every source file the demo's build compiles: components, overlays, the reader, hooks. */
const sources = readdirSync(new URL("../src/", import.meta.url), { recursive: true, encoding: "utf8" })
  .filter((name) => /\.tsx?$/.test(name) && !name.endsWith(".d.ts"))
  .map((name) => `src/${name}`)
  .sort();

/** A compiler event: what happened to one function, where it starts, and why when it gave up. */
interface CompilerEvent {
  kind: string;
  fnLoc?: { start: { line: number }; end: { line: number } } | null;
  detail?: unknown;
}

/**
 * The functions in `path` the compiler gave up on, unless they opt out with "use no memo" and say
 * why, as the overlays' latest-value refs do (the compiler still checks opted-out functions).
 */
async function bailOuts(path: string): Promise<string[]> {
  const text = readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
  const lines = text.split("\n");
  const events: CompilerEvent[] = [];
  const logger = { logEvent: (_: string, event: CompilerEvent) => events.push(event) };
  await transformAsync(text, {
    filename: path,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: ["typescript", "jsx"] },
    plugins: [[reactCompiler, { logger }]],
  });
  // The directive opens the function's body: within its first lines, and inside the function.
  const optedOut = ({ start, end }: { start: { line: number }; end: { line: number } }) =>
    lines.slice(start.line - 1, Math.min(start.line + 6, end.line)).some((l) => l.trim() === '"use no memo";');
  return events
    .filter((event) => event.kind === "CompileError" || event.kind === "PipelineError")
    .filter((event) => event.fnLoc == null || !optedOut(event.fnLoc))
    .map((event) => `line ${event.fnLoc?.start.line ?? "?"}: ${JSON.stringify(event.detail).slice(0, 300)}`);
}

describe("the React Compiler", () => {
  it.each(sources)("compiles every component in %s, or it opts out and says why", async (path) => {
    expect(await bailOuts(path)).toEqual([]);
  });
});
