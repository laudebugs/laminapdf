// What every checker shares: `node checkers/<id>.mjs <project-dir>` loads the @laminapdf/sdk the
// scratch project installed, runs the task's checks, runs `qpdf --check` on each output PDF
// (skipped with a note when qpdf isn't installed), and prints one line: `PASS: …` (exit 0) or
// `FAIL: …` (exit 1).
import { Buffer } from "node:buffer";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { inflateSync } from "node:zlib";

export { taskInput } from "../tasks.mjs";

/** A failed check: its message is the one-line reason. */
export class CheckFailure extends Error {}

/**
 * Fails the check with `reason` unless `condition` holds.
 *
 * @param {unknown} condition
 * @param {string} reason
 * @returns {asserts condition}
 */
export function expect(condition, reason) {
  if (!condition) throw new CheckFailure(reason);
}

/**
 * Whether `a` and `b` differ by at most `tolerance`.
 *
 * @param {number} a
 * @param {number} b
 * @param {number} [tolerance]
 */
export function near(a, b, tolerance = 0.5) {
  return Math.abs(a - b) <= tolerance;
}

/**
 * @typedef {object} Context
 * @property {string} project - the scratch project's folder
 * @property {any} sdk - the project's @laminapdf/sdk, initialised with the text module
 * @property {(name: string) => Uint8Array} read - a project file's bytes (fails the check when missing)
 * @property {(name: string) => any} open - opens a project PDF (fails the check when it doesn't open)
 * @property {(name: string) => void} pdf - marks a project file as an output PDF for `qpdf --check`
 */

/**
 * Loads @laminapdf/sdk from the scratch project's node_modules (not this repository's) and
 * initialises it with the text module, which can also extract text.
 *
 * @param {string} project
 */
async function loadSdk(project) {
  const entry = createRequire(join(project, "package.json")).resolve("@laminapdf/sdk");
  const sdk = await import(pathToFileURL(entry).href);
  await sdk.init({ text: true });
  return sdk;
}

/**
 * @param {string} project
 * @param {any} sdk
 * @param {string[]} pdfs - collects the output PDFs to check with qpdf
 * @returns {Context}
 */
function context(project, sdk, pdfs) {
  const read = (/** @type {string} */ name) => {
    const path = join(project, name);
    expect(existsSync(path), `${name} is missing`);
    return new Uint8Array(readFileSync(path));
  };
  const open = (/** @type {string} */ name) => {
    try {
      return sdk.Document.open(read(name));
    } catch (e) {
      if (e instanceof CheckFailure) throw e;
      throw new CheckFailure(`${name} doesn't open: ${sdk.isLaminaPdfError(e) ? e.code : String(e)}`);
    }
  };
  return { project, sdk, read, open, pdf: (name) => pdfs.push(name) };
}

/**
 * `qpdf --check` on each file: `undefined` when all pass, else the first failure; a note when
 * qpdf isn't installed.
 *
 * @param {string} project
 * @param {string[]} pdfs
 * @returns {{ failure?: string, note?: string }}
 */
export function qpdfCheck(project, pdfs) {
  if (pdfs.length === 0) return {};
  const version = spawnSync("qpdf", ["--version"], { encoding: "utf8" });
  if (version.error) return { note: "qpdf not installed: --check skipped" };
  for (const name of pdfs) {
    const run = spawnSync("qpdf", ["--check", join(project, name)], { encoding: "utf8" });
    if (run.status !== 0) {
      const line = `${run.stdout}${run.stderr}`.trim().split("\n").at(-1) ?? "";
      return { failure: `qpdf --check ${name} exited ${run.status}: ${line}` };
    }
  }
  return {};
}

/**
 * Runs a task's checks and exits: 0 with `PASS: <summary>`, 1 with `FAIL: <reason>`.
 *
 * @param {(ctx: Context) => Promise<string> | string} checks - returns a one-line summary
 */
export async function check(checks) {
  const project = process.argv[2];
  if (!project) {
    console.log("FAIL: usage: node checkers/<id>.mjs <project-dir>");
    process.exit(1);
  }
  /** @type {string[]} */
  const pdfs = [];
  try {
    const summary = await checks(context(project, await loadSdk(project), pdfs));
    const qpdf = qpdfCheck(project, pdfs);
    if (qpdf.failure) throw new CheckFailure(qpdf.failure);
    console.log(`PASS: ${summary}${qpdf.note ? ` (${qpdf.note})` : ""}`);
    process.exit(0);
  } catch (e) {
    const reason = e instanceof CheckFailure ? e.message : `checker error: ${e instanceof Error ? e.message : e}`;
    console.log(`FAIL: ${reason.replaceAll("\n", " ")}`);
    process.exit(1);
  }
}

/** Whitespace collapsed to single spaces, trimmed. */
export function squash(/** @type {string} */ text) {
  return text.replace(/\s+/g, " ").trim();
}

/**
 * Whether `prefix` is the start of `bytes` (an incremental update keeps the original bytes).
 *
 * @param {Uint8Array} bytes
 * @param {Uint8Array} prefix
 */
export function startsWith(bytes, prefix) {
  if (bytes.length < prefix.length) return false;
  return prefix.every((b, i) => bytes[i] === b);
}

/**
 * The file's bytes as Latin-1 text, followed by every stream that inflates (object streams,
 * content, XMP), so a search sees dictionaries inside compressed object streams too. A crude
 * scan, enough for the structure the SDK can't read back (outlines, links, tagging).
 *
 * @param {Uint8Array} bytes
 * @returns {string}
 */
export function pdfObjects(bytes) {
  const raw = Buffer.from(bytes).toString("latin1");
  const parts = [raw];
  const start = /stream\r?\n/g;
  for (let m = start.exec(raw); m; m = start.exec(raw)) {
    const from = m.index + m[0].length;
    const end = raw.indexOf("endstream", from);
    if (end < 0) break;
    try {
      parts.push(inflateSync(bytes.subarray(from, end), { finishFlush: 2 }).toString("latin1"));
    } catch {
      // Not Flate (JPEG, uncompressed or damaged): the raw text above covers it.
    }
  }
  return parts.join("\n");
}

/**
 * Whether a PDF text string with this value appears in `objects`: as a literal string, or as
 * UTF-16BE with a byte-order mark, literal or hexadecimal.
 *
 * @param {string} objects - from {@link pdfObjects}
 * @param {string} value
 */
export function hasPdfString(objects, value) {
  const utf16 = Buffer.concat([Buffer.from([0xfe, 0xff]), Buffer.from(value, "utf16le").swap16()]);
  const hex = utf16.toString("hex");
  const lower = objects.toLowerCase();
  return (
    objects.includes(`(${value})`) ||
    objects.includes(utf16.toString("latin1")) ||
    lower.includes(`<${hex}>`) ||
    lower.includes(`<${Buffer.from(value, "latin1").toString("hex")}>`)
  );
}

/**
 * The page's text runs with their text and bounds (page space: top-left, y down).
 *
 * @param {any} doc
 * @param {number} index
 * @returns {{ text: string, x: number, y: number, width: number, height: number }[]}
 */
export function runs(doc, index) {
  return doc
    .page(index)
    .text()
    .runs.map((/** @type {any} */ r) => ({ text: r.text, ...r.bounds }));
}

/**
 * Runs a Node script in the project with `args`; its stdout, stderr and exit status.
 *
 * @param {string} project
 * @param {string} script
 * @param {string[]} args
 */
export function runNode(project, script, args) {
  const run = spawnSync(process.execPath, [script, ...args], { cwd: project, encoding: "utf8", timeout: 60_000 });
  return { stdout: run.stdout ?? "", stderr: run.stderr ?? "", status: run.status };
}
