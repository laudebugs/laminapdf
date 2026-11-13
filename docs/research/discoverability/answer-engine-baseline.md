# Answer-engine baseline: does OpenAI with web search recommend Lamina PDF?

- **Status:** Baseline run on 2026-11-13
- **Asked for by:** the [discoverability plan](../../plans/discoverability.md) (todo W3i)
- **Tool:** [`tools/answer-check/`](../../../tools/answer-check/README.md)

This note records what an AI answer engine says when a developer asks it for a PDF library or reader. On 2026-11-13, none of 60 answers from OpenAI's `gpt-6.1-sol` with web search named Lamina PDF, and none of their searches returned one of its pages. The run asked twenty questions that don't name Lamina PDF, three times each. Later runs, after W3d and W3e ship, are compared with it here.

## Settings

The check's `config.json` and `questions.json` as committed with this note, run with `--engine openai`: model `gpt-6.1-sol` through the Responses API at reasoning effort `medium`, at most 16,000 output tokens, the `web_search` tool with at most five tool calls per answer and its sources included, three runs per question, the question alone with no system prompt.

The baseline asks OpenAI rather than Claude because the session that ran it could reach OpenAI's API with the owner's key but not Anthropic's. A later run compares with this baseline only if it asks OpenAI with the same model and settings; a run of Claude starts a baseline of its own.

## Results

The run on 2026-11-13 used 1,244,532 input tokens, 45,624 output tokens and 90 searches, about $3.90 at OpenAI's list prices. Every answer completed. The searches returned 2,871 pages across the 60 answers: 23 for the answer that saw fewest, 108 for the one that saw most.

Of 60 answers to 20 questions, 0 name Lamina PDF, 0 saw one of its pages in their search results, and 0 cite one. None of the 2,871 pages was Lamina PDF's, so the gap is in what search finds, not in what the model chooses to say. W3d and W3e, which make the pages easier to find and index, are the changes the next run measures.

| Question | Runs | Named | In results | Cited | Failed | By fallback | Most-cited hosts |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ts-generate | 3 | 0 | 0 | 0 | 0 | 0 | pdf-lib.js.org, pptr.dev, react-pdf.org |
| rust-generate | 3 | 0 | 0 | 0 | 0 | 0 | docs.rs, github.com |
| browser-generate | 3 | 0 | 0 | 0 | 0 | 0 | developer.mozilla.org, github.com, pdf-lib.js.org |
| workers-pdf | 3 | 0 | 0 | 0 | 0 | 0 | developers.cloudflare.com, pdf-lib.js.org |
| wasm-pdf | 3 | 0 | 0 | 0 | 0 | 0 | pdfium.js.org, github.com, v2.embedpdf.com |
| edit-pdf-js | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdf-lib.js.org |
| signed-pdf-edit | 3 | 0 | 0 | 0 | 0 | 0 | helpx.adobe.com, adobe.com |
| accessible-pdf | 3 | 0 | 0 | 0 | 0 | 0 | docs.verapdf.org, doc.courtbouillon.org, itextpdf.com |
| pdfa | 3 | 0 | 0 | 0 | 0 | 0 | itextpdf.com, svn.apache.org, pdfbox.apache.org |
| permissive-licence | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdfbox.apache.org, pdfium.googlesource.com |
| pdf-lib-alternative | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdfkit.org, pptr.dev |
| rag-ts | 3 | 0 | 0 | 0 | 0 | 0 | raw.githubusercontent.com, github.com, docs.langchain.com |
| rag-rust | 3 | 0 | 0 | 0 | 0 | 0 | docs.rs, github.com, manpages.debian.org |
| pdf-to-markdown | 3 | 0 | 0 | 0 | 0 | 0 | pymupdf.readthedocs.io, docling.ai |
| pdf-tables | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pymupdf.readthedocs.io, camelot-py.readthedocs.io |
| pdf-cli-json | 3 | 0 | 0 | 0 | 0 | 0 | github.com, mupdf.readthedocs.io, arxiv.org |
| pdf-mcp | 3 | 0 | 0 | 0 | 0 | 0 | github.com |
| failed-redaction | 3 | 0 | 0 | 0 | 0 | 0 | helpx.adobe.com, mupdf.readthedocs.io, adobe.com |
| private-reader | 3 | 0 | 0 | 0 | 0 | 0 | support.mozilla.org, pdfgear.com, simplepdf.com |
| web-viewer | 3 | 0 | 0 | 0 | 0 | 0 | github.com, nutrient.io, pdfobject.com |

The most-cited hosts show where the answers come from instead: projects' own documentation sites (pdf-lib.js.org, docs.rs, pdfbox.apache.org), GitHub repositories, and vendor help pages for the questions about signing and redaction.
