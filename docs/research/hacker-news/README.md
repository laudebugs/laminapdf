# Hacker News research: what PDF users want

What ten years of Hacker News say PDF users want from readers, editors, extraction libraries and generators, and what lets them down. The data covers 3,755 stories and 56,907 comments posted between 1 November 2016 and 6 November 2026. Researched 6 November 2026.

| Document | What it is |
| --- | --- |
| [pdf-users-on-hacker-news.md](pdf-users-on-hacker-news.md) | **The report.** The data at a glance, six patterns across the decade, what each area asks for, ten recommendations (R1–R10) mapped to todos, four things to avoid, and the limits. |
| [notes/readers.md](notes/readers.md) | Readers: tablets and e-ink, annotation, reflow, speed, forms without Acrobat; security, bloat, upsells and AI prompts; SumatraPDF, Preview, pdf.js, Acrobat over time. |
| [notes/editors.md](notes/editors.md) | Editors and document jobs: signing, annotating, forms, OCR, page tools, redaction; Acrobat's price, Linux gaps, XFA, saves that break files, fake redactions. |
| [notes/extraction.md](notes/extraction.md) | Extraction and parsing: tables, LLM-ready output, scans, cost, reading order, bounding boxes; broken tables, invented text, fonts, slowness; the shift to vision models. |
| [notes/generation.md](notes/generation.md) | Generation: Markdown and templates in, maths, charts, speed, page control, tagged output; Typst against LaTeX, the weight of headless Chrome, AGPL, pagination from HTML. |
| [notes/format-and-security.md](notes/format-and-security.md) | The format, security and trust: safe opening, PDF/A, licences, signatures, redaction; exploits, phones, accessibility, active content, forgery, forensics. |
| [notes/method.md](notes/method.md) | How the data was collected, sorted into areas and counted; the spot checks, quote checks and limits. |

Method: scripts in [`tools/hn-research/`](../../../tools/hn-research/README.md) collected the data through the HN Search API and counted themes with regular expressions. One agent read each area, wrote and spot-checked the theme patterns, and drew out the evidence; a script checked every quote against its comment. The data stays out of git; the scripts fetch it again. The plan that turns this into work is [`docs/plans/trust-and-proof.md`](../../plans/trust-and-proof.md).
