// What one answer says about Lamina PDF, and the report over every answer (todo W3i). Pure
// functions over the Messages API's content blocks, so test/report.test.mjs can check them on
// recorded answers without the network.

/**
 * @typedef {object} Target - what counts as Lamina PDF (config.json's `target`)
 * @property {string[]} names - mentions, matched without case in the answer's text
 * @property {string[]} domains - a URL on this host or a subdomain of it is Lamina PDF's
 * @property {string[]} urlPrefixes - any other URL that starts with one of these is
 *
 * @typedef {object} Answer - what the runner keeps of one answer
 * @property {string} question - the question's id
 * @property {number} run - 1-based
 * @property {string} model - the model that served the answer (a fallback can differ)
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
 * Whether `url` is one of Lamina PDF's.
 *
 * @param {string} url
 * @param {Target} target
 * @returns {boolean}
 */
export function isTargetUrl(url, target) {
  let host;
  try {
    host = new URL(url).hostname.toLowerCase();
  } catch {
    return false;
  }
  if (target.domains.some((domain) => host === domain || host.endsWith(`.${domain}`))) return true;
  return target.urlPrefixes.some(
    (prefix) => url === prefix || url.startsWith(prefix.endsWith("/") ? prefix : `${prefix}/`),
  );
}

/**
 * Whether `text` names Lamina PDF: one of the target's names, without case, not inside a longer word.
 *
 * @param {string} text
 * @param {Target} target
 * @returns {boolean}
 */
export function mentions(text, target) {
  const lower = text.toLowerCase();
  return target.names.some((name) => {
    const needle = name.toLowerCase();
    for (let at = lower.indexOf(needle); at !== -1; at = lower.indexOf(needle, at + 1)) {
      const before = lower[at - 1] ?? " ";
      const after = lower[at + needle.length] ?? " ";
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
 * @param {{ question: string, run: number, model: string, stopReason: string | null, blocks: any[], usage?: Usage }} raw
 * @param {Target} target
 * @returns {Answer}
 */
export function analyse(
  { question, run, model, stopReason, blocks, usage = { inputTokens: 0, outputTokens: 0, searches: 0 } },
  target,
) {
  /** @type {Parts} */
  const parts = { text: [], queries: [], results: [], cited: [], searchErrors: [] };
  for (const block of blocks) READERS[block.type]?.(block, parts);
  const text = parts.text.join("");
  const results = unique(parts.results);
  const cited = unique(parts.cited);
  return {
    question,
    run,
    model,
    stopReason,
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

/** @typedef {{ text: string[], queries: string[], results: string[], cited: string[], searchErrors: string[] }} Parts */

/**
 * What each kind of content block adds to an answer; other kinds add nothing.
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
};

/**
 * Per question, how many runs mentioned, surfaced and cited Lamina PDF, and the hosts its answers
 * cited most.
 *
 * @param {Answer[]} answers
 * @returns {{ question: string, runs: number, mentioned: number, surfaced: number, cited: number, failed: number, topHosts: string[] }[]}
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
      failed: runs.filter((answer) => answer.stopReason === "refusal" || answer.searchErrors.length > 0).length,
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
 * @param {{ date: string, settings: Record<string, unknown>, answers: Answer[] }} report
 * @returns {string}
 */
export function markdown({ date, settings, answers }) {
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
    `Of ${answers.length} answers to ${rows.length} questions, ${total("mentioned")} name Lamina PDF, ${total("surfaced")} saw one of its pages in their search results, and ${total("cited")} cite one.`,
    "",
    "| Question | Runs | Named | In results | Cited | Failed | Most-cited hosts |",
    "| --- | --- | --- | --- | --- | --- | --- |",
    ...rows.map(
      (row) =>
        `| ${row.question} | ${row.runs} | ${row.mentioned} | ${row.surfaced} | ${row.cited} | ${row.failed} | ${row.topHosts.join(", ") || "—"} |`,
    ),
    "",
  ];
  return lines.join("\n");
}
