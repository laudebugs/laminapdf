// The report over recorded answers, without the network: `node --test tools/answer-check/test/*.test.mjs`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { analyse, isTargetUrl, markdown, mentions, summarise } from "../report.mjs";
import { parseArgs, request, selectQuestions } from "../run.mjs";

const config = JSON.parse(readFileSync(new URL("../config.json", import.meta.url), "utf8"));
const target = config.target;

/** A recorded answer: a search, its results, then text that cites one of them. */
const answerBlocks = [
  { type: "text", text: "Let me look that up." },
  {
    type: "server_tool_use",
    id: "srvtoolu_1",
    name: "web_search",
    input: { query: "typescript pdf generation library" },
  },
  {
    type: "web_search_tool_result",
    tool_use_id: "srvtoolu_1",
    content: [
      { type: "web_search_result", url: "https://pdf-lib.js.org/", title: "PDF-LIB", encrypted_content: "…" },
      {
        type: "web_search_result",
        url: "https://docs.laminapdf.com/start/first-pdf/",
        title: "Your first PDF",
        encrypted_content: "…",
      },
      { type: "web_search_result", url: "https://pdf-lib.js.org/", title: "PDF-LIB (again)", encrypted_content: "…" },
    ],
  },
  {
    type: "text",
    text: "Try pdf-lib or Lamina PDF.",
    citations: [
      { type: "web_search_result_location", url: "https://pdf-lib.js.org/", title: "PDF-LIB", cited_text: "…" },
      { type: "web_search_result_location", url: "https://www.github.com/x/y", title: "y", cited_text: "…" },
    ],
  },
];

describe("isTargetUrl", () => {
  it("takes the domain and its subdomains, and the listed URL prefixes", () => {
    assert.equal(isTargetUrl("https://laminapdf.com/", target), true);
    assert.equal(isTargetUrl("https://docs.laminapdf.com/guides/x/", target), true);
    assert.equal(isTargetUrl("https://www.npmjs.com/package/@laminapdf/sdk", target), true);
    assert.equal(isTargetUrl("https://crates.io/crates/laminapdf", target), true);
  });

  it("ignores the scheme, www, the query, the fragment, a trailing slash and case", () => {
    assert.equal(isTargetUrl("http://npmjs.com/package/@laminapdf/sdk", target), true);
    assert.equal(isTargetUrl("https://github.com/laudebugs/laminapdf#readme", target), true);
    assert.equal(isTargetUrl("https://github.com/LaudeBugs/LaminaPDF/?tab=readme", target), true);
  });

  it("refuses look-alikes and other projects", () => {
    assert.equal(isTargetUrl("https://notlaminapdf.com/", target), false);
    assert.equal(isTargetUrl("https://crates.io/crates/laminapdf-unrelated-fork", target), false);
    assert.equal(isTargetUrl("https://github.com/laudebugs/laminapdfx", target), false);
    assert.equal(isTargetUrl("not a url", target), false);
  });
});

describe("mentions", () => {
  it("finds a name without case, inside punctuation", () => {
    assert.equal(mentions("Consider lamina pdf, or…", target), true);
    assert.equal(mentions("npm i @laminapdf/sdk", target), true);
    assert.equal(mentions("(LaminaPDF)", target), true);
  });

  it("reads hyphens, underscores and line breaks as spaces", () => {
    assert.equal(mentions("Use Lamina-PDF here", target), true);
    assert.equal(mentions("Use Lamina\nPDF here", target), true);
  });

  it("refuses a name inside a longer word", () => {
    assert.equal(mentions("laminapdfx is something else", target), false);
    assert.equal(mentions("A laminate PDF cover", target), false);
  });
});

describe("analyse", () => {
  const answer = analyse(
    { question: "ts-generate", run: 1, model: "claude-opus-5-5", stopReason: "end_turn", blocks: answerBlocks },
    target,
  );

  it("joins the text and finds the name", () => {
    assert.equal(answer.text, "Let me look that up.Try pdf-lib or Lamina PDF.");
    assert.equal(answer.mentioned, true);
  });

  it("keeps the queries, the results without repeats and the citations", () => {
    assert.deepEqual(answer.queries, ["typescript pdf generation library"]);
    assert.deepEqual(answer.results, ["https://pdf-lib.js.org/", "https://docs.laminapdf.com/start/first-pdf/"]);
    assert.deepEqual(answer.cited, ["https://pdf-lib.js.org/", "https://www.github.com/x/y"]);
  });

  it("counts Lamina PDF in the results but not in the citations", () => {
    assert.equal(answer.surfaced, true);
    assert.equal(answer.citedTarget, false);
  });

  it("takes the URLs of other tool results, such as a fetched page", () => {
    const fetched = analyse(
      {
        question: "q",
        run: 1,
        model: "m",
        stopReason: "end_turn",
        blocks: [
          {
            type: "web_fetch_tool_result",
            tool_use_id: "t",
            content: { type: "web_fetch_result", url: "https://laminapdf.com/" },
          },
        ],
      },
      target,
    );
    assert.equal(fetched.surfaced, true);
  });

  it("records a failed search's error code, not results", () => {
    const failed = analyse(
      {
        question: "q",
        run: 1,
        model: "m",
        stopReason: "end_turn",
        blocks: [
          {
            type: "web_search_tool_result",
            tool_use_id: "t",
            content: { type: "web_search_tool_result_error", error_code: "max_uses_exceeded" },
          },
        ],
      },
      target,
    );
    assert.deepEqual(failed.searchErrors, ["max_uses_exceeded"]);
    assert.deepEqual(failed.results, []);
  });
});

describe("the report", () => {
  const answers = [
    analyse(
      { question: "ts-generate", run: 1, model: "claude-opus-5-5", stopReason: "end_turn", blocks: answerBlocks },
      target,
    ),
    analyse({ question: "ts-generate", run: 2, model: "claude-opus-5-5", stopReason: "refusal", blocks: [] }, target),
  ];

  it("counts per question", () => {
    assert.deepEqual(summarise(answers), [
      {
        question: "ts-generate",
        runs: 2,
        mentioned: 1,
        surfaced: 1,
        cited: 0,
        failed: 1,
        fallback: 0,
        topHosts: ["github.com", "pdf-lib.js.org"],
      },
    ]);
  });

  it("counts an unfinished turn and a fallback answer", () => {
    const rows = summarise([
      analyse({ question: "q", run: 1, model: "m", stopReason: "pause_turn", blocks: [] }, target),
      analyse({ question: "q", run: 2, model: "other", fallback: true, stopReason: "end_turn", blocks: [] }, target),
    ]);
    assert.equal(rows[0]?.failed, 1);
    assert.equal(rows[0]?.fallback, 1);
  });

  it("says when a run stopped early", () => {
    const text = markdown({ date: "2026-11-11", settings: {}, answers, stopped: "overloaded" });
    assert.match(text, /The run stopped early:\*\* overloaded/);
  });

  it("writes the totals and a row per question", () => {
    const text = markdown({ date: "2026-11-11", settings: { model: "claude-opus-5-5" }, answers });
    assert.match(text, /Of 2 answers to 1 questions, 1 name Lamina PDF, 1 saw one of its pages/);
    assert.match(text, /\| ts-generate \| 2 \| 1 \| 1 \| 0 \| 1 \| 0 \| github\.com, pdf-lib\.js\.org \|/);
  });
});

describe("the runner", () => {
  it("asks the question alone, with web search and the pinned model", () => {
    const params = request(config, "What PDF library?");
    assert.deepEqual(params.messages, [{ role: "user", content: "What PDF library?" }]);
    assert.equal(params.model, config.model);
    assert.deepEqual(params.tools, [{ type: "web_search_20260209", name: "web_search", max_uses: config.maxSearches }]);
    assert.equal("system" in params, false);
  });

  it("reads its options and refuses unknown ones", () => {
    assert.deepEqual(parseArgs(["--questions", "a,b", "--repeats", "2", "--dry-run"]), {
      questions: ["a", "b"],
      repeats: 2,
      dryRun: true,
    });
    assert.throws(() => parseArgs(["--repeats", "0"]), /whole number/);
    assert.throws(() => parseArgs(["--nope"]), /unknown argument/);
    assert.throws(() => parseArgs(["--questions", ""]), /one or more question ids/);
  });

  it("selects questions by id and refuses an unknown one", () => {
    const all = [
      { id: "a", area: "x", question: "A?" },
      { id: "b", area: "x", question: "B?" },
    ];
    assert.deepEqual(selectQuestions(all, undefined), all);
    assert.deepEqual(
      selectQuestions(all, ["b"]).map((q) => q.id),
      ["b"],
    );
    assert.throws(() => selectQuestions(all, ["c"]), /no question with the id c/);
  });

  it("has unique question ids, none naming Lamina PDF", () => {
    const questions = JSON.parse(readFileSync(new URL("../questions.json", import.meta.url), "utf8"));
    assert.equal(new Set(questions.map((q) => q.id)).size, questions.length);
    for (const q of questions) assert.equal(mentions(q.question, target), false, q.id);
  });
});
