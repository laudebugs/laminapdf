// What the site's pages show from the repository's own files, read at build time, so nothing on a
// page is typed by hand: the first PDF in TypeScript and Rust (examples CI runs and checks against
// their goldens), the command-line tool's example commands (its README), the docs site's install
// lines and package table (the install page), and the licence policy (Cargo.toml and deny.toml).

import { withVersion } from "@laminapdf/site-kit/version";
import cargoToml from "../../../../Cargo.toml?raw";
import helloRs from "../../../../crates/laminapdf/examples/hello.rs?raw";
import cliReadme from "../../../../crates/laminapdf-cli/README.md?raw";
import denyToml from "../../../../deny.toml?raw";
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

/** The names a `use` line brings in: `Error` for `use std::error::Error;`, each name in braces. */
function importedNames(line: string): string[] {
  const path = line.replace(/^use\s+/, "").replace(/;\s*$/, "");
  const list = path.match(/\{([^}]*)\}/)?.[1] ?? path.split("::").at(-1) ?? "";
  return list
    .split(",")
    .map(
      (name) =>
        name
          .trim()
          .split(/\s+as\s+/)
          .at(-1) ?? "",
    )
    .filter((name) => name !== "");
}

/**
 * The Rust example's `build` function with the doc comment above it
 * (`crates/laminapdf/examples/hello.rs`), and the `use` lines it needs: the standard library's,
 * then Lamina PDF's. `main`, which only writes the file, and its imports are left out.
 */
export function rustFirstPdf(source: string = helloRs): string {
  const lines = source.split("\n");
  const build = lines.findIndex((line) => line.startsWith("pub(crate) fn build("));
  if (build < 0) throw new Error("hello.rs: no `build` function");
  let start = build;
  while (start > 0 && lines[start - 1]?.startsWith("///")) start -= 1;
  const body = lines.slice(start).join("\n").trimEnd();
  const uses = lines.filter(
    (line) => line.startsWith("use ") && importedNames(line).some((name) => new RegExp(`\\b${name}\\b`).test(body)),
  );
  const std = uses.filter((line) => line.startsWith("use std::"));
  const others = uses.filter((line) => !line.startsWith("use std::"));
  if (others.length === 0) throw new Error("hello.rs: `build` uses nothing from a `use laminapdf::` line");
  return [...std, ...(std.length > 0 ? [""] : []), ...others, "", body].join("\n");
}

/**
 * The commands of the CLI README's `console` examples that succeed: each `$ …` line but `echo $?`
 * and those whose output is an `error[…]`, which the README shows to explain the error. A trailing
 * `# …` comment keeps two spaces before it, not the README's column alignment.
 */
export function cliCommands(readme: string = cliReadme): string {
  const blocks = [...readme.matchAll(/```console\n([\s\S]*?)```/g)].map(([, block = ""]) => block);
  const commands = blocks.flatMap((block) =>
    block
      .split(/^(?=\$ )/m)
      .filter((example) => example.startsWith("$ ") && !example.startsWith("$ echo $?"))
      .filter((example) => !/^error\[/m.test(example))
      .map((example) => (example.split("\n")[0] ?? "").replace(/\s{2,}#/, "  #")),
  );
  if (commands.length === 0) throw new Error("laminapdf-cli README: no `$ ` commands in its console examples");
  return commands.join("\n");
}

/**
 * The install page's install commands, `npm install …` and `cargo add …`, with the version the page
 * writes as `{{version}}` filled in, as the docs site fills it in.
 */
export function installLines(page: string = installPage): readonly string[] {
  const lines = page.split("\n").filter((line) => /^(npm install|cargo add) /.test(line));
  if (lines.length !== 2) throw new Error("install.mdx: expected one `npm install` and one `cargo add` line");
  return lines.map((line) => withVersion(line, VERSION));
}

/**
 * The install page's note on versions: the sentences from "Both follow SemVer" to the end of
 * their paragraph, so the home page repeats the install page's advice word for word.
 */
export function versionNote(page: string = installPage): string {
  const note = page.match(/Both follow SemVer[^\n]*/)?.[0];
  if (note === undefined) throw new Error('install.mdx: no "Both follow SemVer" note');
  return note.trim();
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

/** A licence allowed for some named dependencies only, with those dependencies. */
interface LicenceException {
  readonly licence: string;
  readonly crates: readonly string[];
}

/** The licence policy, as `Cargo.toml` and `deny.toml` state it. */
export interface LicencePolicy {
  /** Lamina PDF's own licence (`[workspace.package] license`). */
  readonly licence: string;
  /** The licences every dependency may have (`[licenses] allow`). */
  readonly allowed: readonly string[];
  /** Licences allowed for named crates only (`[licenses] exceptions`), grouped by licence. */
  readonly exceptions: readonly LicenceException[];
  /** Crates refused whatever their metadata says (`[bans] deny`), with the reason given. */
  readonly banned: readonly { readonly name: string; readonly reason: string }[];
}

/** The text of `[section]` in a TOML file, up to the next table header. */
function section(toml: string, name: string): string {
  const start = toml.indexOf(`\n[${name}]\n`);
  if (start < 0) throw new Error(`no [${name}] table`);
  const body = toml.slice(start + name.length + 4);
  const next = body.search(/^\[/m);
  return next < 0 ? body : body.slice(0, next);
}

/** The value of `key = [ … ]` in `table`, with its comment lines removed. */
function array(table: string, key: string): string {
  const match = table.match(new RegExp(`^${key} = \\[([\\s\\S]*?)^\\]`, "m"));
  if (match === null) throw new Error(`no \`${key} = [\` array`);
  return (match[1] ?? "")
    .split("\n")
    .filter((line) => !line.trim().startsWith("#"))
    .join("\n");
}

/**
 * The inline tables (`{ name = "…", allow = ["…"] }`) of an array's text, each as its keys and
 * values, whatever their order: a string value as is, an array of strings as a list.
 */
function entries(array: string): Record<string, string | string[]>[] {
  return [...array.matchAll(/\{([^}]*)\}/g)].map(([, body = ""]) => {
    const fields: Record<string, string | string[]> = {};
    for (const [, key = "", text, list] of body.matchAll(/([\w-]+)\s*=\s*(?:"([^"]*)"|\[([^\]]*)\])/g)) {
      fields[key] = text ?? [...(list ?? "").matchAll(/"([^"]+)"/g)].map(([, item = ""]) => item);
    }
    return fields;
  });
}

/** A deny.toml entry's crate: its `name`, or its `crate` (cargo-deny's newer spelling, `name@version` allowed). */
function crateOf(entry: Record<string, string | string[]>, table: string): string {
  const name = entry.name ?? entry.crate;
  if (typeof name !== "string") throw new Error(`deny.toml: an entry of ${table} names no crate`);
  return name.split("@")[0] ?? name;
}

/** The policy `cargo` and `deny` (the root manifest and `deny.toml`) state. */
export function licencePolicy(cargo: string = cargoToml, deny: string = denyToml): LicencePolicy {
  const licence = section(cargo, "workspace.package").match(/^license = "([^"]+)"/m)?.[1];
  if (licence === undefined) throw new Error("Cargo.toml: no [workspace.package] license");
  const licences = section(deny, "licenses");
  const allowed = [...array(licences, "allow").matchAll(/"([^"]+)"/g)].map(([, name = ""]) => name);
  const byLicence = new Map<string, string[]>();
  for (const entry of entries(array(licences, "exceptions"))) {
    const crate = crateOf(entry, "[licenses] exceptions");
    const allow = entry.allow;
    if (!Array.isArray(allow) || allow.length === 0)
      throw new Error(`deny.toml: the exception for ${crate} allows nothing`);
    for (const id of allow) byLicence.set(id, [...(byLicence.get(id) ?? []), crate]);
  }
  const exceptions = [...byLicence].map(([id, crates]) => ({ licence: id, crates }));
  const banned = entries(array(section(deny, "bans"), "deny")).map((entry) => ({
    name: crateOf(entry, "[bans] deny"),
    reason: typeof entry.reason === "string" ? entry.reason : "",
  }));
  if (allowed.length === 0 || banned.length === 0) throw new Error("deny.toml: no allowed licences or no bans");
  return { licence, allowed, exceptions, banned };
}
