// What one answer says about Lamina PDF, and the report over every answer (todo W3i). Pure
// functions over an answer's parts: the Claude Messages API's content blocks or the OpenAI
// Responses API's output items, so test/report.test.mjs can check them on recorded answers without
// the network.

/**
 * @typedef {object} Target - what counts as Lamina PDF (config.json's `target`)
 * @property {string[]} names - mentions, matched without case in the answer's text
 * @property {string[]} domains - a URL on this host or a subdomain of it is Lamina PDF's
 * @property {string[]} urlPrefixes - any other URL that starts with one of these is
 *
 * @typedef {object} Answer - what the runner keeps of one answer
 * @property {string} question - the question's id
 * @property {number} run - 1-based
 * @property {string} model - the model that served the answer
 * @property {boolean} fallback - a fallback model served it, not the pinned one
 * @property {string | null} stopReason
 * @property {string} text - the answer's text blocks, joined
 * @property {string[]} queries - what the model searched for, in order
 * @property {string[]} results - every search result's URL, in order, repeats removed
 * @property {string[]} cited - every URL the text cites, in order, repeats removed
 * @property {string[]} searchErrors - error codes of searches that failed
 * @property {boolean} mentioned - the text names Lamina PDF
 * @property {boolean} surfaced - a search result is Lamina PDF's
 * @property {boolean} citedTarget - the text cites a Lamina PDF URL
 * @property {Usage} usage
 *
 * @typedef {{ inputTokens: number, outputTokens: number, searches: number }} Usage - summed over the turn's responses
 */

/**
 * A URL as host and path, compared without the scheme, a leading `www.`, the query, the fragment,
 * a trailing slash or case, or `null` if it isn't a URL.
 *
 * @param {string} url
 * @returns {{ host: string, path: string } | null}
 */
function place(url) {
  try {
    const parsed = new URL(url);
    return {
      host: parsed.hostname.toLowerCase().replace(/^www\./, ""),
      path: parsed.pathname.toLowerCase().replace(/\/+$/, ""),
    };
  } catch {
    return null;
  }
}

/**
 * Whether `url` is one of Lamina PDF's: on one of the target's domains or a subdomain, or at or
 * below one of its URL prefixes.
 *
 * @param {string} url
 * @param {Target} target
 * @returns {boolean}
 */
export function isTargetUrl(url, target) {
  const at = place(url);
  if (at === null) return false;
  if (target.domains.some((domain) => at.host === domain || at.host.endsWith(`.${domain}`))) return true;
  return target.urlPrefixes.some((prefix) => {
    const base = place(prefix);
    return base !== null && at.host === base.host && (at.path === base.path || at.path.startsWith(`${base.path}/`));
  });
}

/** Lower case, with hyphens, underscores and runs of white space each read as one space. */
const fold = (/** @type {string} */ text) => text.toLowerCase().replace(/[\s_-]+/g, " ");

/**
 * Whether `text` names Lamina PDF: one of the target's names, without case, read through
 * {@link fold} (so "Lamina-PDF" and a line break inside the name count), not inside a longer word.
 *
 * @param {string} text
 * @param {Target} target
 * @returns {boolean}
 */
export function mentions(text, target) {
  const folded = fold(text);
  return target.names.some((name) => {
    const needle = fold(name);
    for (let at = folded.indexOf(needle); at !== -1; at = folded.indexOf(needle, at + 1)) {
      const before = folded[at - 1] ?? " ";
      const after = folded[at + needle.length] ?? " ";
      if (!/[a-z0-9]/.test(before) && (needle.endsWith("/") || !/[a-z0-9]/.test(after))) return true;
    }
    return false;
  });
}

/** @param {string[]} list */
const unique = (list) => [...new Set(list)];

/**
 * One answer from the content blocks of every response in its turn (a paused turn continues in
 * a second response).
 *
 * @param {{ question: string, run: number, model: string, fallback?: boolean, stopReason: string | null, blocks: any[], usage?: Usage }} raw
 * @param {Target} target
 * @returns {Answer}
 */
export function analyse(
  {
    question,
    run,
    model,
    fallback = false,
    stopReason,
    blocks,
    usage = { inputTokens: 0, outputTokens: 0, searches: 0 },
  },
  target,
) {
  /** @type {Parts} */
  const parts = { text: [], queries: [], results: [], cited: [], searchErrors: [] };
  for (const block of blocks) {
    const reader = READERS[block.type];
    if (reader) reader(block, parts);
    else if (typeof block.type === "string" && block.type.endsWith("_tool_result"))
      urlsIn(block.content, parts.results);
  }
  const text = parts.text.join("");
  const results = unique(parts.results);
  const cited = unique(parts.cited);
  return {
    question,
    run,
    model,
    fallback,
    stopReason: parts.refused ? "refusal" : stopReason,
    text,
    queries: parts.queries,
    results,
    cited,
    searchErrors: parts.searchErrors,
    mentioned: mentions(text, target),
    surfaced: results.some((url) => isTargetUrl(url, target)),
    citedTarget: cited.some((url) => isTargetUrl(url, target)),
    usage,
  };
}

/** @typedef {{ text: string[], queries: string[], results: string[], cited: string[], searchErrors: string[], refused?: boolean }} Parts */

/**
 * What each kind of part adds to an answer: Claude's content blocks (`text`, `server_tool_use`,
 * `web_search_tool_result`) and OpenAI's output items (`message`, `web_search_call`), whose type
 * names don't overlap. Other kinds add nothing.
 *
 * @type {Record<string, (block: any, parts: Parts) => void>}
 */
const READERS = {
  text(block, parts) {
    parts.text.push(block.text);
    for (const citation of block.citations ?? []) if (typeof citation.url === "string") parts.cited.push(citation.url);
  },
  server_tool_use(block, parts) {
    if (block.name === "web_search" && typeof block.input?.query === "string") parts.queries.push(block.input.query);
  },
  // A list of results on success, one error object on failure.
  web_search_tool_result(block, parts) {
    if (Array.isArray(block.content)) {
      for (const result of block.content) if (typeof result.url === "string") parts.results.push(result.url);
    } else if (block.content?.error_code) {
      parts.searchErrors.push(block.content.error_code);
    }
  },
  // OpenAI: the answer's text with its URL citations, or a refusal.
  message(block, parts) {
    for (const content of block.content ?? []) {
      if (content.type === "output_text") {
        parts.text.push(content.text);
        for (const note of content.annotations ?? []) {
          if (note.type === "url_citation" && typeof note.url === "string") parts.cited.push(note.url);
        }
      } else if (content.type === "refusal") {
        parts.refused = true;
      }
    }
  },
  // OpenAI: a search (its queries and, when asked for, its sources) or a page it opened.
  web_search_call(block, parts) {
    const action = block.action ?? {};
    for (const query of [action.query, ...(action.queries ?? [])]) {
      if (typeof query === "string" && !parts.queries.includes(query)) parts.queries.push(query);
    }
    for (const source of action.sources ?? []) if (typeof source.url === "string") parts.results.push(source.url);
    if (typeof action.url === "string") parts.results.push(action.url);
    if (block.status === "failed") parts.searchErrors.push("failed");
  },
};

/**
 * Every `url` string anywhere inside a value: the results of tool blocks other than web search
 * (a fetched page, the dynamic filtering's code run) that may carry the pages the model saw.
 *
 * @param {unknown} value
 * @param {string[]} into
 */
function urlsIn(value, into) {
  if (Array.isArray(value)) for (const item of value) urlsIn(item, into);
  else if (value !== null && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) {
      if (key === "url" && typeof item === "string") into.push(item);
      else urlsIn(item, into);
    }
  }
}

/**
 * Why an answer counts as failed: a refusal, an unfinished turn, a cut-off text or a failed search.
 * Claude's stop reasons, then OpenAI's: an `incomplete` response's reason, or a `failed` one.
 */
const UNFINISHED = new Set(["refusal", "pause_turn", "max_tokens", "max_output_tokens", "content_filter", "failed"]);

/**
 * Per question, how many runs mentioned, surfaced and cited Lamina PDF, and the hosts its answers
 * cited most.
 *
 * @param {Answer[]} answers
 * @returns {{ question: string, runs: number, mentioned: number, surfaced: number, cited: number, failed: number, fallback: number, topHosts: string[] }[]}
 */
export function summarise(answers) {
  /** @type {Map<string, Answer[]>} */
  const byQuestion = new Map();
  for (const answer of answers) byQuestion.set(answer.question, [...(byQuestion.get(answer.question) ?? []), answer]);
  return [...byQuestion].map(([question, runs]) => {
    const hosts = new Map();
    for (const url of runs.flatMap((answer) => answer.cited)) {
      try {
        const host = new URL(url).hostname.replace(/^www\./, "");
        hosts.set(host, (hosts.get(host) ?? 0) + 1);
      } catch {
        // A citation that isn't a URL counts for no host.
      }
    }
    return {
      question,
      runs: runs.length,
      mentioned: runs.filter((answer) => answer.mentioned).length,
      surfaced: runs.filter((answer) => answer.surfaced).length,
      cited: runs.filter((answer) => answer.citedTarget).length,
      failed: runs.filter((answer) => UNFINISHED.has(answer.stopReason ?? "") || answer.searchErrors.length > 0).length,
      fallback: runs.filter((answer) => answer.fallback).length,
      topHosts: [...hosts]
        .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
        .slice(0, 3)
        .map(([host]) => host),
    };
  });
}

/**
 * The report as Markdown: the settings, the totals, then one row per question.
 *
 * @param {{ date: string, settings: Record<string, unknown>, answers: Answer[], stopped?: string }} report
 * @returns {string}
 */
export function markdown({ date, settings, answers, stopped }) {
  const rows = summarise(answers);
  const total = (/** @type {"mentioned" | "surfaced" | "cited"} */ key) => rows.reduce((sum, row) => sum + row[key], 0);
  const models = unique(answers.map((answer) => answer.model)).join(", ");
  const lines = [
    `# Answer-engine check, ${date}`,
    "",
    `Settings: ${Object.entries(settings)
      .map(([key, value]) => `${key} ${JSON.stringify(value)}`)
      .join(", ")}. Answers served by: ${models || "none"}.`,
    "",
    `Used ${answers.reduce((sum, answer) => sum + answer.usage.inputTokens, 0)} input tokens, ${answers.reduce((sum, answer) => sum + answer.usage.outputTokens, 0)} output tokens and ${answers.reduce((sum, answer) => sum + answer.usage.searches, 0)} searches.`,
    "",
    ...(stopped ? [`**The run stopped early:** ${stopped}. These are the answers before it.`, ""] : []),
    `Of ${answers.length} answers to ${rows.length} questions, ${total("mentioned")} name Lamina PDF, ${total("surfaced")} saw one of its pages in their search results, and ${total("cited")} cite one.`,
    "",
    "| Question | Runs | Named | In results | Cited | Failed | By fallback | Most-cited hosts |",
    "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ...rows.map(
      (row) =>
        `| ${row.question} | ${row.runs} | ${row.mentioned} | ${row.surfaced} | ${row.cited} | ${row.failed} | ${row.fallback} | ${row.topHosts.join(", ") || "—"} |`,
    ),
    "",
  ];
  return lines.join("\n");
}
