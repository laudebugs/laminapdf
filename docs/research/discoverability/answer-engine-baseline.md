# Answer-engine baseline: does OpenAI with web search recommend Lamina PDF?

- **Status:** Baseline run on 2026-11-13
- **Asked for by:** the [discoverability plan](../../plans/discoverability.md) (todo W3i)
- **Tool:** [`tools/answer-check/`](../../../tools/answer-check/README.md)

This note records what an AI answer engine says when a developer asks it for a PDF library or reader. On 2026-11-13, none of 60 answers from OpenAI's `gpt-6.1-sol` with web search named Lamina PDF, and none of their searches returned one of its pages. The run asked twenty questions that don't name Lamina PDF, three times each. Later runs, after W3d and W3e ship, are compared with it here.

## Settings

The check's `config.json` and `questions.json` as committed with this note, run with `--engine openai`: model `gpt-6.1-sol` through the Responses API, the `web_search` tool with at most five searches per answer and its sources included, three runs per question, the question alone with no system prompt.

The baseline asks OpenAI rather than Claude because the session that ran it could reach OpenAI's API with the owner's key but not Anthropic's. A later run compares with this baseline only if it asks OpenAI with the same model and settings; a run of Claude starts a baseline of its own.

## Results

The run on 2026-11-13 used 1,285,314 input tokens, 47,077 output tokens and 150 searches, about $4.50 at OpenAI's list prices. Every answer completed.

Of 60 answers to 20 questions, 0 name Lamina PDF, 0 saw one of its pages in their search results, and 0 cite one. The engine never reached Lamina PDF's pages, so the gap is in what search finds, not in what the model chooses to say. W3d and W3e, which make the pages easier to find and index, are the changes the next run measures.

| Question | Runs | Named | In results | Cited | Failed | By fallback | Most-cited hosts |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ts-generate | 3 | 0 | 0 | 0 | 0 | 0 | pdf-lib.js.org, react-pdf.org, github.com |
| rust-generate | 3 | 0 | 0 | 0 | 0 | 0 | docs.rs, github.com |
| browser-generate | 3 | 0 | 0 | 0 | 0 | 0 | developer.mozilla.org, github.com, pdf-lib.js.org |
| workers-pdf | 3 | 0 | 0 | 0 | 0 | 0 | developers.cloudflare.com, pdf-lib.js.org, github.com |
| wasm-pdf | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdfium.js.org, embedpdf.com |
| edit-pdf-js | 3 | 0 | 0 | 0 | 0 | 0 | pdf-lib.js.org, github.com |
| signed-pdf-edit | 3 | 0 | 0 | 0 | 0 | 0 | adobe.com, helpx.adobe.com |
| accessible-pdf | 3 | 0 | 0 | 0 | 0 | 0 | docs.verapdf.org, doc.courtbouillon.org, pdfkit.org |
| pdfa | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdfbox.apache.org, apache.googlesource.com |
| permissive-licence | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdfbox.apache.org, pdfium.googlesource.com |
| pdf-lib-alternative | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pdfkit.org, pdfmake.github.io |
| rag-ts | 3 | 0 | 0 | 0 | 0 | 0 | github.com, raw.githubusercontent.com, aws.amazon.com |
| rag-rust | 3 | 0 | 0 | 0 | 0 | 0 | docs.rs, github.com, pdf.oxide.fyi |
| pdf-to-markdown | 3 | 0 | 0 | 0 | 0 | 0 | pymupdf.readthedocs.io, github.com, pypi.org |
| pdf-tables | 3 | 0 | 0 | 0 | 0 | 0 | camelot-py.readthedocs.io, github.com, ocrmypdf.readthedocs.io |
| pdf-cli-json | 3 | 0 | 0 | 0 | 0 | 0 | github.com |
| pdf-mcp | 3 | 0 | 0 | 0 | 0 | 0 | github.com, pypi.org |
| failed-redaction | 3 | 0 | 0 | 0 | 0 | 0 | helpx.adobe.com, ico.org.uk, mupdf.readthedocs.io |
| private-reader | 3 | 0 | 0 | 0 | 0 | 0 | github.com, learn.microsoft.com, readhere.app |
| web-viewer | 3 | 0 | 0 | 0 | 0 | 0 | github.com, apryse.com, pdfobject.com |

The most-cited hosts show where the answers come from instead: projects' own documentation sites (pdf-lib.js.org, docs.rs, pdfbox.apache.org), GitHub repositories, and vendor help pages for the questions about signing and redaction.
