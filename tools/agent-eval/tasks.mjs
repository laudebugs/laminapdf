// The agent eval's task files (tasks/<id>.md): front matter, then the prompt the agent gets.
// Shared by the runner and the checkers, so both read inputs the same way.
//
//   ---
//   id: rotate-page-3
//   title: Rotate page 3 and save incrementally
//   checker: rotate-page-3.mjs
//   inputs: [fixtures/golden/links-and-outlines.pdf => report.pdf]
//   output: rotated.pdf
//   ---
//   <prompt>
import { readdirSync, readFileSync } from "node:fs";
import { basename } from "node:path";

/** The repository root. */
export const ROOT = new URL("../../", import.meta.url);
/** The folder of task files. */
export const TASKS = new URL("tasks/", import.meta.url);

/**
 * @typedef {object} TaskInput
 * @property {string} from - the source file, relative to the repository root
 * @property {string} to - its name in the scratch project
 *
 * @typedef {object} Task
 * @property {string} id
 * @property {string} title
 * @property {string} checker - the checker's file name in checkers/
 * @property {TaskInput[]} inputs
 * @property {string} output - the file (or script) the agent must leave in the project
 * @property {string} prompt
 */

const REQUIRED = ["id", "title", "checker", "output"];

/**
 * One input entry: `fixtures/a.pdf => b.pdf`, or `fixtures/a.pdf` (keeps its file name).
 *
 * @param {string} entry
 * @returns {TaskInput}
 */
export function parseInput(entry) {
  const [from, to] = entry.split("=>").map((s) => s.trim());
  if (!from) throw new Error(`empty input entry: "${entry}"`);
  return { from, to: to || basename(from) };
}

/**
 * A front-matter value: `[a, b]` is a list, anything else a string.
 *
 * @param {string} raw
 * @returns {string | string[]}
 */
function parseValue(raw) {
  const value = raw.trim();
  if (!value.startsWith("[")) return value;
  if (!value.endsWith("]")) throw new Error(`unclosed list: ${value}`);
  const inner = value.slice(1, -1).trim();
  return inner === "" ? [] : inner.split(",").map((s) => s.trim());
}

/**
 * The `key: value` lines between the `---` fences.
 *
 * @param {string} block
 * @returns {Record<string, string | string[]>}
 */
function parseFields(block) {
  /** @type {Record<string, string | string[]>} */
  const fields = {};
  for (const line of block.split("\n")) {
    if (line.trim() === "" || line.trimStart().startsWith("#")) continue;
    const colon = line.indexOf(":");
    if (colon < 1) throw new Error(`front matter line without a key: "${line}"`);
    fields[line.slice(0, colon).trim()] = parseValue(line.slice(colon + 1));
  }
  return fields;
}

/**
 * Parses a task file.
 *
 * @param {string} text - the file's contents
 * @param {string} [name] - its file name, for messages and the id check
 * @returns {Task}
 */
export function parseTask(text, name = "task") {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/.exec(text);
  if (!match) throw new Error(`${name}: no front matter`);
  const fields = parseFields(match[1] ?? "");
  for (const key of REQUIRED) {
    if (typeof fields[key] !== "string" || fields[key] === "") throw new Error(`${name}: missing "${key}"`);
  }
  const inputs = fields.inputs ?? [];
  const prompt = (match[2] ?? "").trim();
  if (prompt === "") throw new Error(`${name}: empty prompt`);
  return {
    id: String(fields.id),
    title: String(fields.title),
    checker: String(fields.checker),
    inputs: (Array.isArray(inputs) ? inputs : [inputs]).map(parseInput),
    output: String(fields.output),
    prompt,
  };
}

/**
 * Every task in tasks/, by file name; a file whose name differs from its `id` is an error.
 *
 * @param {URL} [dir]
 * @returns {Task[]}
 */
export function loadTasks(dir = TASKS) {
  const files = readdirSync(dir)
    .filter((f) => f.endsWith(".md"))
    .sort();
  return files.map((file) => {
    const task = parseTask(readFileSync(new URL(file, dir), "utf8"), file);
    if (`${task.id}.md` !== file) throw new Error(`${file}: id "${task.id}" differs from the file name`);
    return task;
  });
}

/**
 * The repository bytes behind a task's input, read from the fixture rather than the scratch
 * project, so an agent that edits its input in place can't change what the checker compares.
 *
 * @param {string} id - the task id
 * @param {string} name - the input's name in the scratch project
 * @returns {Uint8Array}
 */
export function taskInput(id, name) {
  const task = parseTask(readFileSync(new URL(`${id}.md`, TASKS), "utf8"), `${id}.md`);
  const input = task.inputs.find((i) => i.to === name);
  if (!input) throw new Error(`task ${id} has no input named ${name}`);
  return new Uint8Array(readFileSync(new URL(input.from, ROOT)));
}
