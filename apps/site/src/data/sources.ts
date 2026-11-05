// What the home page shows from the repository's own files, read at build time, so nothing on the
// page is typed by hand: the first PDF in TypeScript and Rust (examples CI runs and checks against
// their goldens), the command-line tool's example commands (its README), and the docs site's
// install lines and package table (the install page).
import helloRs from "../../../../crates/laminapdf/examples/hello.rs?raw";
import cliReadme from "../../../../crates/laminapdf-cli/README.md?raw";
import helloTs from "../../../../packages/sdk/examples/hello.ts?raw";
import sdk from "../../../../packages/sdk/package.json";
import installPage from "../../../docs/src/content/docs/install.mdx?raw";

/** One published package, as the docs site's install table lists it. */
export interface Package {
  readonly name: string;
  readonly registry: string;
  readonly url: string;
  readonly holds: string;
}

/** The version every published package shares (ADR 0017's lockstep version). */
export const VERSION: string = sdk.version;

/** `text` without its leading comment lines and the blank lines after them. */
function withoutHeader(text: string): string {
  const lines = text.split("\n");
  const start = lines.findIndex((line) => !line.startsWith("//") && line.trim() !== "");
  return lines.slice(start).join("\n").trimEnd();
}

/** The TypeScript example (`packages/sdk/examples/hello.ts`) without its header comment. */
export function typescriptFirstPdf(source: string = helloTs): string {
  return withoutHeader(source);
}

/**
 * The Rust example's `use laminapdf::…` line and its `build` function with the doc comment above
 * it (`crates/laminapdf/examples/hello.rs`), leaving out `main`, which only writes the file.
 */
export function rustFirstPdf(source: string = helloRs): string {
  const lines = source.split("\n");
  const use = lines.find((line) => line.startsWith("use laminapdf::"));
  const build = lines.findIndex((line) => line.startsWith("pub(crate) fn build("));
  if (use === undefined || build < 0) throw new Error("hello.rs: no `use laminapdf::` line or `build` function");
  let start = build;
  while (start > 0 && lines[start - 1]?.startsWith("///")) start -= 1;
  return [use, "", ...lines.slice(start)].join("\n").trimEnd();
}

/** The commands of the CLI README's `console` examples (`$ …` lines, without `echo $?`). */
export function cliCommands(readme: string = cliReadme): string {
  const blocks = [...readme.matchAll(/```console\n([\s\S]*?)```/g)].map(([, block = ""]) => block);
  const commands = blocks
    .flatMap((block) => block.split("\n"))
    .filter((line) => line.startsWith("$ ") && !line.startsWith("$ echo $?"));
  if (commands.length === 0) throw new Error("laminapdf-cli README: no `$ ` commands in its console examples");
  return commands.join("\n");
}

/** The install page's install commands: `npm install …` and `cargo add …`. */
export function installLines(page: string = installPage): readonly string[] {
  const lines = page.split("\n").filter((line) => /^(npm install|cargo add) /.test(line));
  if (lines.length !== 2) throw new Error("install.mdx: expected one `npm install` and one `cargo add` line");
  return lines;
}

/** The install page's table of published packages, in its order. */
export function packages(page: string = installPage): readonly Package[] {
  const rows = page
    .split("\n")
    .map((line) => line.match(/^\| `([^`]+)` \| \[([^\]]+)\]\(([^)]+)\) \| (.+) \|$/))
    .filter((match) => match !== null);
  if (rows.length === 0) throw new Error("install.mdx: no package table");
  return rows.map(([, name = "", registry = "", url = "", holds = ""]) => ({ name, registry, url, holds }));
}
