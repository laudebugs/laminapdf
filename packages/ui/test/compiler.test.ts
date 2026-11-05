import { readdirSync, readFileSync } from "node:fs";
import { transformAsync } from "@babel/core";
import reactCompiler from "babel-plugin-react-compiler";
import { describe, expect, it } from "vitest";

/** Every component file of the chrome: what the demo page's React Compiler build memoises. */
const FOLDERS = ["components", "overlays", "reader", "platform", "state"];
const sources = FOLDERS.flatMap((folder) =>
  readdirSync(new URL(`../src/${folder}/`, import.meta.url))
    .filter((name) => name.endsWith(".tsx"))
    .map((name) => `src/${folder}/${name}`),
);

/** A compiler event: what happened to one function, where it starts, and why when it gave up. */
interface CompilerEvent {
  kind: string;
  fnLoc?: { start: { line: number } } | null;
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
  const optedOut = (line: number) => lines.slice(line - 1, line + 6).some((l) => l.includes('"use no memo"'));
  return events
    .filter((event) => event.kind === "CompileError" || event.kind === "PipelineError")
    .filter((event) => event.fnLoc == null || !optedOut(event.fnLoc.start.line))
    .map((event) => `line ${event.fnLoc?.start.line ?? "?"}: ${JSON.stringify(event.detail).slice(0, 300)}`);
}

describe("the React Compiler", () => {
  it.each(sources)("compiles every component in %s, or it opts out and says why", async (path) => {
    expect(await bailOuts(path)).toEqual([]);
  });
});
