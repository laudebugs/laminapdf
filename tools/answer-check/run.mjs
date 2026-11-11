// The answer-engine check (todo W3i): asks each question in questions.json, `repeats` times, to
// Claude with web search, and records whether each answer names Lamina PDF, whether its search
// results held one of Lamina PDF's pages, and whether it cites one. Recorded by hand, never in CI:
// it needs a paid API key, and answers vary from run to run.
//
//   node tools/answer-check/run.mjs [--questions a,b] [--repeats N] [--dry-run]
//
// Results: target/answer-check/<date>/answers.json (every answer, with its text, searches, results
// and citations) and report.md (report.mjs's table).
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { analyse, markdown } from "./report.mjs";

const HERE = fileURLToPath(new URL(".", import.meta.url));
const ROOT = fileURLToPath(new URL("../../", import.meta.url));
/** Opts the request into a fallback model when the pinned model declines the question. */
const FALLBACK_BETA = "server-side-fallback-2026-07-01";

/**
 * @typedef {object} Config
 * @property {string} model
 * @property {"low" | "medium" | "high" | "xhigh" | "max"} effort
 * @property {number} maxTokens
 * @property {number} maxSearches - the web search tool's `max_uses` per answer
 * @property {number} repeats - how many times each question is asked
 * @property {number} maxContinuations - how many times a paused turn is resumed
 * @property {import("./report.mjs").Target} target
 *
 * @typedef {{ id: string, area: string, question: string }} Question
 */

/**
 * The command line's options.
 *
 * @param {string[]} argv - the arguments after the script
 * @returns {{ questions: string[] | undefined, repeats: number | undefined, dryRun: boolean }}
 */
export function parseArgs(argv) {
  const options = { questions: undefined, repeats: undefined, dryRun: false };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dry-run") options.dryRun = true;
    else if (arg === "--questions") options.questions = questionIds(argv[++i]);
    else if (arg === "--repeats") options.repeats = repeatCount(argv[++i]);
    else throw new Error(`unknown argument ${arg} (try --questions a,b, --repeats N or --dry-run)`);
  }
  return options;
}

/** @param {string | undefined} value - `--questions`' value */
function questionIds(value) {
  const ids = (value ?? "").split(",").filter(Boolean);
  if (ids.length === 0) throw new Error("--questions takes one or more question ids");
  return ids;
}

/** @param {string | undefined} value - `--repeats`' value */
function repeatCount(value) {
  const count = Number(value);
  if (!Number.isInteger(count) || count < 1) throw new Error("--repeats takes a whole number of 1 or more");
  return count;
}

/**
 * The request for one question: the question alone, as a person would ask it, with web search.
 *
 * @param {Config} config
 * @param {string} question
 * @returns {Record<string, unknown>}
 */
export function request(config, question) {
  return {
    model: config.model,
    max_tokens: config.maxTokens,
    output_config: { effort: config.effort },
    betas: [FALLBACK_BETA],
    fallbacks: "default",
    tools: [{ type: "web_search_20260209", name: "web_search", max_uses: config.maxSearches }],
    messages: [{ role: "user", content: question }],
  };
}

/** A folder under target/answer-check/ for today's run, `-2` and so on for later runs that day. */
function outputDir() {
  const date = new Date().toISOString().slice(0, 10);
  const base = join(ROOT, "target", "answer-check");
  let dir = join(base, date);
  for (let n = 2; existsSync(dir); n += 1) dir = join(base, `${date}-${n}`);
  return { date, dir };
}

/**
 * Asks one question once, resuming a turn the server paused (its web searches hit the server's
 * loop limit), and returns every content block of the turn.
 *
 * @param {import("@anthropic-ai/sdk").default} client
 * @param {Config} config
 * @param {Record<string, unknown>} params
 */
async function ask(client, config, params) {
  const messages = [.../** @type {any[]} */ (params.messages)];
  const blocks = [];
  const usage = { inputTokens: 0, outputTokens: 0, searches: 0 };
  for (let turn = 0; ; turn += 1) {
    const response = await client.beta.messages.create(/** @type {any} */ ({ ...params, messages }));
    blocks.push(...response.content);
    const { input_tokens, cache_read_input_tokens, cache_creation_input_tokens } = response.usage;
    usage.inputTokens += input_tokens + (cache_read_input_tokens ?? 0) + (cache_creation_input_tokens ?? 0);
    usage.outputTokens += response.usage.output_tokens;
    usage.searches += response.usage.server_tool_use?.web_search_requests ?? 0;
    if (response.stop_reason !== "pause_turn" || turn >= config.maxContinuations) {
      return {
        model: response.model,
        fallback: response.model !== config.model,
        stopReason: response.stop_reason,
        blocks,
        usage,
      };
    }
    messages.push({ role: "assistant", content: response.content });
  }
}

/**
 * The questions to ask: every one, or those `ids` name.
 *
 * @param {Question[]} all
 * @param {string[] | undefined} ids
 * @returns {Question[]}
 */
export function selectQuestions(all, ids) {
  if (ids === undefined) return all;
  const unknown = ids.filter((id) => !all.some((question) => question.id === id));
  if (unknown.length > 0) throw new Error(`no question with the id ${unknown.join(", ")}`);
  return all.filter((question) => ids.includes(question.id));
}

/** @param {import("./report.mjs").Answer} answer */
function describe(answer) {
  const named = answer.mentioned ? "named" : "not named";
  const surfaced = answer.surfaced ? "in results" : "not in results";
  const cited = answer.citedTarget ? "cited" : "not cited";
  return `${answer.question} #${answer.run}: ${named}, ${surfaced}, ${cited} (${answer.model}, ${answer.stopReason})`;
}

/**
 * Asks every question `repeats` times, one request at a time, and prints each answer's line.
 *
 * @param {Config} config
 * @param {Question[]} questions
 * @param {number} repeats
 */
async function askAll(config, questions, repeats) {
  const { default: Anthropic } = await import("@anthropic-ai/sdk");
  // Credentials as the SDK reads them: ANTHROPIC_API_KEY, else an `ant auth login` profile.
  const client = new Anthropic();
  const answers = [];
  try {
    for (const question of questions) {
      for (let run = 1; run <= repeats; run += 1) {
        const raw = await ask(client, config, request(config, question.question));
        const answer = analyse({ question: question.id, run, ...raw }, config.target);
        answers.push(answer);
        console.log(describe(answer));
      }
    }
  } catch (error) {
    // Keep the answers already paid for: the caller writes them, then fails.
    return { answers, stopped: error instanceof Error ? error.message : String(error) };
  }
  return { answers, stopped: undefined };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  /** @type {Config} */
  const config = JSON.parse(readFileSync(join(HERE, "config.json"), "utf8"));
  const questions = selectQuestions(JSON.parse(readFileSync(join(HERE, "questions.json"), "utf8")), options.questions);
  const repeats = options.repeats ?? config.repeats;

  if (options.dryRun) {
    console.log(
      `${questions.length} questions × ${repeats} runs = ${questions.length * repeats} answers; each request:`,
    );
    for (const question of questions) console.log(JSON.stringify(request(config, question.question)));
    return;
  }

  const { answers, stopped } = await askAll(config, questions, repeats);
  const { date, dir } = outputDir();
  mkdirSync(dir, { recursive: true });
  const settings = {
    model: config.model,
    effort: config.effort,
    maxSearches: config.maxSearches,
    repeats,
    tool: "web_search_20260209",
  };
  writeFileSync(join(dir, "answers.json"), `${JSON.stringify({ date, settings, stopped, answers }, null, 2)}\n`);
  writeFileSync(join(dir, "report.md"), markdown({ date, settings, answers, stopped }));
  console.log(`Wrote ${join(dir, "report.md")}`);
  if (stopped) throw new Error(`the run stopped early after ${answers.length} answers: ${stopped}`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  });
}
