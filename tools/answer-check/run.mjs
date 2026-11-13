// The answer-engine check (todo W3i): asks each question in questions.json, `repeats` times, to an
// answer engine with web search (Claude, or OpenAI's Responses API), and records whether each
// answer names Lamina PDF, whether its search results held one of Lamina PDF's pages, and whether
// it cites one. Recorded by hand, never in CI: it needs a paid API key, and answers vary from run
// to run.
//
//   node tools/answer-check/run.mjs [--engine claude|openai] [--questions a,b] [--repeats N] [--dry-run]
//
// Results: target/answer-check/<date>-<engine>/answers.json (every answer, with its text, searches,
// results and citations) and report.md (report.mjs's table).
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { analyse, markdown } from "./report.mjs";

const HERE = fileURLToPath(new URL(".", import.meta.url));
const ROOT = fileURLToPath(new URL("../../", import.meta.url));
/** Opts a Claude request into a fallback model when the pinned model declines the question. */
const FALLBACK_BETA = "server-side-fallback-2026-07-01";
/** How many times a request that failed for a passing reason is sent again, as the Anthropic SDK does. */
const RETRIES = 3;
/** The engines the check can ask. */
const ENGINES = ["claude", "openai"];
/** Asks the Responses API for the pages each web search returned; every read of a response needs it. */
const SOURCES = "web_search_call.action.sources";

/**
 * @typedef {object} ClaudeSettings
 * @property {string} model
 * @property {"low" | "medium" | "high" | "xhigh" | "max"} effort
 * @property {number} maxTokens
 * @property {number} maxSearches - the web search tool's `max_uses` per answer
 *
 * @typedef {object} OpenAISettings
 * @property {string} model
 * @property {"low" | "medium" | "high"} effort - the response's `reasoning.effort`
 * @property {number} maxTokens - the response's `max_output_tokens`
 * @property {number} maxSearches - the response's `max_tool_calls`
 *
 * @typedef {object} Config
 * @property {"claude" | "openai"} engine - the default engine
 * @property {{ claude: ClaudeSettings, openai: OpenAISettings }} engines
 * @property {number} repeats - how many times each question is asked
 * @property {number} maxContinuations - how many times a paused Claude turn is resumed
 * @property {import("./report.mjs").Target} target
 *
 * @typedef {{ id: string, area: string, question: string }} Question
 *
 * @typedef {{ model: string, fallback: boolean, stopReason: string | null, blocks: any[], usage: import("./report.mjs").Usage }} Raw
 */

/**
 * The command line's options.
 *
 * @param {string[]} argv - the arguments after the script
 * @returns {{ engine: "claude" | "openai" | undefined, questions: string[] | undefined, repeats: number | undefined, dryRun: boolean }}
 */
export function parseArgs(argv) {
  /** @type {{ engine: "claude" | "openai" | undefined, questions: string[] | undefined, repeats: number | undefined, dryRun: boolean }} */
  const options = { engine: undefined, questions: undefined, repeats: undefined, dryRun: false };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dry-run") options.dryRun = true;
    else if (arg === "--engine") options.engine = engineName(argv[++i]);
    else if (arg === "--questions") options.questions = questionIds(argv[++i]);
    else if (arg === "--repeats") options.repeats = repeatCount(argv[++i]);
    else throw new Error(`unknown argument ${arg} (try --engine openai, --questions a,b, --repeats N or --dry-run)`);
  }
  return options;
}

/** @param {string | undefined} value - `--engine`'s value */
function engineName(value) {
  if (value !== "claude" && value !== "openai")
    throw new Error(`the engine is one of ${ENGINES.join(", ")}, not ${value}`);
  return value;
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
 * The Claude request for one question: the question alone, as a person would ask it, with web
 * search.
 *
 * @param {ClaudeSettings} settings
 * @param {string} question
 * @returns {Record<string, unknown>}
 */
export function claudeRequest(settings, question) {
  return {
    model: settings.model,
    max_tokens: settings.maxTokens,
    output_config: { effort: settings.effort },
    betas: [FALLBACK_BETA],
    fallbacks: "default",
    tools: [{ type: "web_search_20260209", name: "web_search", max_uses: settings.maxSearches }],
    messages: [{ role: "user", content: question }],
  };
}

/**
 * The OpenAI Responses API request for one question: the question alone, with web search and its
 * sources included, so the report sees every page the search returned, in background mode (see
 * askOpenAI).
 *
 * @param {OpenAISettings} settings
 * @param {string} question
 * @returns {Record<string, unknown>}
 */
export function openaiRequest(settings, question) {
  return {
    model: settings.model,
    reasoning: { effort: settings.effort },
    max_output_tokens: settings.maxTokens,
    tools: [{ type: "web_search" }],
    max_tool_calls: settings.maxSearches,
    include: [SOURCES],
    background: true,
    input: question,
  };
}

/** A folder under target/answer-check/ for today's run of an engine, `-2` and so on for later runs that day. */
function outputDir(/** @type {string} */ engine) {
  const date = new Date().toISOString().slice(0, 10);
  const base = join(ROOT, "target", "answer-check");
  let dir = join(base, `${date}-${engine}`);
  for (let n = 2; existsSync(dir); n += 1) dir = join(base, `${date}-${engine}-${n}`);
  return { date, dir };
}

/**
 * Asks Claude one question once, resuming a turn the server paused (its web searches hit the
 * server's loop limit), and returns every content block of the turn.
 *
 * @param {import("@anthropic-ai/sdk").default} client
 * @param {Config} config
 * @param {Record<string, unknown>} params
 * @returns {Promise<Raw>}
 */
async function askClaude(client, config, params) {
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
        fallback: response.model !== config.engines.claude.model,
        stopReason: response.stop_reason,
        blocks,
        usage,
      };
    }
    messages.push({ role: "assistant", content: response.content });
  }
}

/**
 * What one OpenAI response says, in the shape the report reads: its output items as the answer's
 * parts, its status as the stop reason (an incomplete response's reason), and its usage.
 *
 * @param {any} response - the Responses API's response object
 * @returns {Raw}
 */
export function openaiRaw(response) {
  const output = Array.isArray(response.output) ? response.output : [];
  const stopReason =
    response.status === "incomplete" ? (response.incomplete_details?.reason ?? "incomplete") : response.status;
  return {
    model: response.model,
    fallback: false,
    stopReason,
    blocks: output,
    usage: {
      inputTokens: response.usage?.input_tokens ?? 0,
      outputTokens: response.usage?.output_tokens ?? 0,
      // Searches only: a web_search_call item may also open a page or find text in one.
      searches: output.filter(
        (/** @type {any} */ item) => item.type === "web_search_call" && item.action?.type === "search",
      ).length,
    },
  };
}

/**
 * Asks OpenAI one question once. The request runs in background mode, so no one HTTP request lasts
 * as long as the answer takes (a proxy can cut a long one off, as the first baseline run found):
 * the POST returns at once with the response's id, and the response is fetched until it is done.
 * The key comes from OPENAI_API_KEY; a proxy that adds the key itself takes a placeholder there.
 *
 * @param {Record<string, unknown>} params
 * @returns {Promise<Raw>}
 */
async function askOpenAI(params) {
  const key = process.env.OPENAI_API_KEY;
  if (!key) throw new Error("set OPENAI_API_KEY to ask OpenAI");
  // The POST is sent once: a retry after a lost answer could start a second response, billed too.
  const created = await callOnce(key, "POST", "responses", params);
  if (created.body === undefined) throw new Error(created.error);
  let body = created.body;
  const started = Date.now();
  try {
    while (body.status === "queued" || body.status === "in_progress") {
      if (Date.now() - started > ANSWER_TIMEOUT_MS) {
        throw new Error(`OpenAI's response ${body.id} took over ${ANSWER_TIMEOUT_MS / 60_000} minutes`);
      }
      await sleep(POLL_MS);
      // The sources must be asked for again on every read: a response read without them has none.
      body = await callWithRetries(key, "GET", `responses/${body.id}?include[]=${SOURCES}`);
    }
  } catch (error) {
    // Stop the response, so it isn't billed after the run gives up on it.
    await callOnce(key, "POST", `responses/${body.id}/cancel`);
    throw error;
  }
  // A failed or cancelled response (no credit left, a server error) stops the run, as a failed call
  // does, so a run never records a string of empty answers as if the engine had answered.
  if (body.status === "failed" || body.status === "cancelled") {
    throw new Error(`OpenAI's response ${body.id} ${body.status}: ${body.error?.message ?? "no message"}`);
  }
  return openaiRaw(body);
}

/** How long to wait between reads of a response in progress, for its whole answer, and for one call. */
const POLL_MS = 3000;
const ANSWER_TIMEOUT_MS = 10 * 60 * 1000;
const CALL_TIMEOUT_MS = 60 * 1000;

/** @param {number} ms */
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * One read of the Responses API, sent again up to {@link RETRIES} times, 2, 4 and 8 s apart, when it
 * fails for a passing reason. Only reads: they change nothing, so a retry is safe.
 *
 * @param {string} key
 * @param {"GET"} method
 * @param {string} path - under https://api.openai.com/v1/
 * @returns {Promise<any>} the response's JSON
 */
async function callWithRetries(key, method, path) {
  for (let attempt = 0; ; attempt += 1) {
    const outcome = await callOnce(key, method, path);
    if (outcome.body !== undefined) return outcome.body;
    if (!outcome.retry || attempt >= RETRIES) throw new Error(outcome.error);
    await sleep(2000 * 2 ** attempt);
  }
}

/**
 * One call to the Responses API: its JSON body, or why it failed and whether a retry may help (the
 * network, a call over a minute, a timeout, a rate limit or a server error, whatever the body).
 *
 * @param {string} key
 * @param {"GET" | "POST"} method
 * @param {string} path
 * @param {Record<string, unknown>} [params]
 * @returns {Promise<{ body: any, retry?: undefined, error?: undefined } | { body?: undefined, retry: boolean, error: string }>}
 */
async function callOnce(key, method, path, params) {
  let response;
  let text;
  try {
    response = await fetch(`https://api.openai.com/v1/${path}`, {
      method,
      headers: { authorization: `Bearer ${key}`, "content-type": "application/json" },
      signal: AbortSignal.timeout(CALL_TIMEOUT_MS),
      ...(params === undefined ? {} : { body: JSON.stringify(params) }),
    });
    text = await response.text();
  } catch (error) {
    return { retry: true, error: `OpenAI unreachable: ${error instanceof Error ? error.message : String(error)}` };
  }
  const passing = response.status === 408 || response.status === 429 || response.status >= 500;
  let body;
  try {
    body = JSON.parse(text);
  } catch {
    return { retry: passing, error: `OpenAI answered ${response.status} with no JSON: ${text.slice(0, 80)}` };
  }
  if (!response.ok) {
    return { retry: passing, error: `OpenAI answered ${response.status}: ${body.error?.message ?? "no message"}` };
  }
  return { body };
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
 * The function that asks one question once on `engine`.
 *
 * @param {Config} config
 * @param {"claude" | "openai"} engine
 * @returns {Promise<(question: string) => Promise<Raw>>}
 */
async function asker(config, engine) {
  if (engine === "openai") return (question) => askOpenAI(request(config, engine, question));
  const { default: Anthropic } = await import("@anthropic-ai/sdk");
  // Credentials as the SDK reads them: ANTHROPIC_API_KEY, else an `ant auth login` profile.
  const client = new Anthropic();
  return (question) => askClaude(client, config, request(config, engine, question));
}

/**
 * The request for one question on `engine`.
 *
 * @param {Config} config
 * @param {"claude" | "openai"} engine
 * @param {string} question
 */
function request(config, engine, question) {
  return engine === "openai"
    ? openaiRequest(config.engines.openai, question)
    : claudeRequest(config.engines.claude, question);
}

/**
 * Asks every question `repeats` times, one request at a time, and prints each answer's line.
 *
 * @param {Config} config
 * @param {"claude" | "openai"} engine
 * @param {Question[]} questions
 * @param {number} repeats
 */
async function askAll(config, engine, questions, repeats) {
  const ask = await asker(config, engine);
  const answers = [];
  try {
    for (const question of questions) {
      for (let run = 1; run <= repeats; run += 1) {
        const raw = await ask(question.question);
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

/**
 * The settings a report records, so two runs can be compared.
 *
 * @param {Config} config
 * @param {"claude" | "openai"} engine
 * @param {number} repeats
 * @returns {Record<string, unknown>}
 */
export function settingsOf(config, engine, repeats) {
  if (engine === "openai") {
    const { model, effort, maxTokens, maxSearches } = config.engines.openai;
    return { engine, model, effort, maxTokens, maxSearches, repeats, tool: "web_search" };
  }
  const { model, effort, maxTokens, maxSearches } = config.engines.claude;
  return { engine, model, effort, maxTokens, maxSearches, repeats, tool: "web_search_20260209" };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  /** @type {Config} */
  const config = JSON.parse(readFileSync(join(HERE, "config.json"), "utf8"));
  const questions = selectQuestions(JSON.parse(readFileSync(join(HERE, "questions.json"), "utf8")), options.questions);
  const repeats = options.repeats ?? config.repeats;
  const engine = options.engine ?? engineName(config.engine);

  if (options.dryRun) {
    console.log(
      `${questions.length} questions × ${repeats} runs = ${questions.length * repeats} answers from ${engine}; each request:`,
    );
    for (const question of questions) console.log(JSON.stringify(request(config, engine, question.question)));
    return;
  }

  const { answers, stopped } = await askAll(config, engine, questions, repeats);
  const { date, dir } = outputDir(engine);
  mkdirSync(dir, { recursive: true });
  const settings = settingsOf(config, engine, repeats);
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
