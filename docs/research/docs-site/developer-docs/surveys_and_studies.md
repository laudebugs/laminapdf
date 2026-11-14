# Developer documentation: quantitative evidence from surveys, industry reports and academic studies

Scope note: researched 2026-11-11 with a budget of about 15 tool calls. Primary sources were read directly where possible: survey pages, and paper PDFs converted with pdftotext. Every number names its source. Second-hand numbers are flagged **[second-hand]**. Postman, Stack Overflow and DORA are vendor or industry surveys, not peer-reviewed.

## 1. What large developer surveys say about documentation

### Takeaway
Technical documentation is the most-used learning resource in the Stack Overflow survey: 84% in 2024 and 67.8% in 2025, on different questions. AI tools now rank above docs for learning AI-related coding. GitHub's 2017 survey supplies the "93% see incomplete or outdated docs" figure, and DORA ties documentation quality to large gains from engineering practices. I could not confirm a current Postman "lack of documentation" figure from Postman's own page.

### Cited Findings
**Stack Overflow Developer Survey 2025** (49,019 respondents in total)
- Question: "How did you learn to code in the past year? Select all that apply." (33,454 responses, 68.2% of respondents). Results: **Technical documentation 67.8%**, other online resources (search, forums, online communities) 58.7%, Stack Overflow/Stack Exchange 51.4%, videos 50%, **AI code-gen tools or AI-enabled apps 44%**, blogs/podcasts 34.8%, online courses 32.7%, books 30.4%. Stack Overflow's own wording: "Technical documentation is the most used resource for learning, with nearly 68% of respondents using it in the past year." — [SO 2025, Developers](https://survey.stackoverflow.co/2025/developers)
- Docs use is flat across age groups: 18–24 65%, 25–34 70%, 35–44 68.6%, 45–54 67%, 55–64 64.3%. — [SO 2025](https://survey.stackoverflow.co/2025/developers)
- Question: "How did you learn to code for AI in the past year?" (28,172 responses). **AI tools 52.5%** ranked first, other online resources 44.2%, **technical documentation 42.5%**, videos 38.7%, Stack Overflow 17%. — [SO 2025](https://survey.stackoverflow.co/2025/developers)
- AI use. "Do you currently use AI tools in your development process?" (33,662 responses): 84% use or plan to use them (76% the year before); 47.1% use them daily; 50.6% of professional developers use them daily. — [SO 2025, AI](https://survey.stackoverflow.co/2025/ai)
- Trust in AI accuracy (33,244 responses): highly trust 3.1%, somewhat trust 29.6%, somewhat distrust 26.1%, highly distrust 19.6%. Distrust (46%) outweighs trust (33%). — [SO 2025, AI](https://survey.stackoverflow.co/2025/ai)
- AI frustrations (31,476 responses): **"AI solutions that are almost right, but not quite" 66%**; "Debugging AI-generated code is more time-consuming" 45.2%. — [SO 2025, AI](https://survey.stackoverflow.co/2025/ai)
- "In the future, if AI can do most coding tasks, in which situations would you still want to ask another person for help?" (29,163 responses): "When I don't trust AI's answers" 75.3%. — [SO 2025, AI](https://survey.stackoverflow.co/2025/ai)
- Which workflow parts are done with AI. For "Searching for answers", 54.1% chose "currently mostly AI" (11,202 in that group) and 55.8% chose "currently partially AI" (20,991). For "creating or maintaining documentation", 39.6% do not plan to use AI. The page's breakdown is hard to read. — [SO 2025, AI](https://survey.stackoverflow.co/2025/ai)
- AI agents (31,877 responses): about 31% use them (daily 14.1%, weekly 9%, monthly or infrequent 7.8%); 37.9% do not plan to. — [SO 2025, AI](https://survey.stackoverflow.co/2025/ai)

**Stack Overflow Developer Survey 2024**
- Question: "What online resources do you use to learn to code? Select all that apply." (48,822 responses, 74.6%). Results: **Technical documentation 83.9%**, Stack Overflow 80.3%, written tutorials 68.4%, blogs 61.4%, how-to videos 54.2%, video courses 49.9%, online books 46.5%, social media 41.2%, **AI 37%**, interactive tutorials 29.3%. Summary line: "Technical documentation (84%) and Stack Overflow (80%) continue to be the top online resources to learn code." — [SO 2024, Developer profile](https://survey.stackoverflow.co/2024/developer-profile)
- Caveat: the 2024 question (online resources only, among people learning) differs from the 2025 question (all learning sources in the past year). **Do not read 84% → 68% as a decline.**

**GitHub Open Source Survey 2017** (5,500 randomly sampled respondents from more than 3,800 GitHub repositories, plus a separate non-random sample of more than 500; in English, Spanish, Chinese, Japanese and Russian)
- Exact wording: "**Incomplete or outdated documentation is a pervasive problem, observed by 93% of respondents.**" — [opensourcesurvey.org/2017](https://opensourcesurvey.org/2017/)
- "60% of contributors say they rarely or never contribute to documentation." — [same](https://opensourcesurvey.org/2017/)
- "Licenses are by far the most important type of documentation to both users and contributors": 64% say a licence is very important when deciding whether to use a project, and 67% when deciding whether to contribute. — [same](https://opensourcesurvey.org/2017/)
- "Documentation helps create inclusive communities." Contributing guides and codes of conduct are valued more by under-represented groups. Nearly a quarter of the community reads and writes English less than "very well". — [same](https://opensourcesurvey.org/2017/)
- The page does not quote the question wording; the questionnaire is in the data download. — [same](https://opensourcesurvey.org/2017/)

**Postman State of the API**
- 2025 report (7th annual, "over 5,700 developers, architects, and executives"): the readable text says "55% struggling with inconsistent documentation and 34% unable to find existing APIs" (wording not given), "Documenting (58%)" among the most common API activities, and 41% use AI to "generate API documentation". **70% of developers are aware of MCP but only 10% use it regularly** (24% plan to explore it). Chart-only figures were not readable. — [Postman 2025](https://www.postman.com/state-of-api/2025/)
- 2020 report: "lack of documentation" was the top obstacle to consuming APIs at 54.3% — **[second-hand]** via [HackerNoon](https://hackernoon.com/54percent-of-developers-cite-lack-of-documentation-as-the-top-obstacle-to-consuming-apis-2v4w3e30) and [devops.com](https://devops.com/2020-postman-state-of-the-api-reveals-api-investments-will-remain-strong/). Not checked against Postman's PDF.
- Sample sizes from Postman's archive page: 2020 over 13,500; 2021 over 28,000; 2022 and 2023 over 37,000; 2024 over 5,600. — [Postman 2024 / archive](https://www.postman.com/state-of-api/2024/)

**DORA (Google Cloud DevOps Research and Assessment)**
- "Analysis by DORA has found a clear link between documentation quality and organizational performance." Documentation quality drove the implementation of every technical practice studied in 2021 and in 2022. — [dora.dev, Documentation quality](https://dora.dev/capabilities/documentation-quality/)
- 2022 data: the lift in organisational performance from each capability, with below- vs above-average documentation quality: trunk-based development 36% vs **1525%**; continuous integration 34% vs 750%; continuous delivery 63% vs 656%; supply chain security 37% vs 451%; SRE 79% vs 343%; loosely coupled teams 46% vs 313%; version control 27% vs 278%. — [dora.dev](https://dora.dev/capabilities/documentation-quality/)
- DORA measures documentation quality with 8 survey items covering clarity, findability and reliability, among others. These concern internal team documentation, not public product docs. — [dora.dev](https://dora.dev/capabilities/documentation-quality/)

### Inferences
- Docs remain the first-choice reference, but AI is the fastest-growing rival, and it already leads for AI-related topics (52.5% vs 42.5%). A docs site should be readable both by people and by the AI tools that answer for them.
- The "almost right" frustration (66%) and the 75% who turn to a person when they distrust AI point to a role for docs: the authoritative source that AI answers get checked against. Exact, versioned, testable statements gain value.
- DORA's figures describe internal docs and organisational outcomes. They support "documentation quality matters", not specific design choices for a public docs site.

### Gaps
- No primary-source Postman figure for 2023–2025 on "lack of documentation as an obstacle" or "documentation as a factor in choosing an API": the page's charts did not render as text.
- JetBrains State of Developer Ecosystem, SlashData, Write the Docs surveys, and State of JS/CSS: not checked within the call budget. I have no verified numbers on documentation from them.
- DORA 2023 and 2024 documentation results (the 2023 report also reported a documentation effect) were not verified here.

## 2. What academic studies find: problems developers hit

### Takeaway
Across Microsoft, IBM and open-source studies, the most severe documentation problems concern content: missing, ambiguous, wrong or outdated information, rather than presentation. Code examples matter most, yet they are also the content most often wrong. Several studies show developers abandoning an API because of its documentation.

### Cited Findings
- **Robillard & DeLine, "A field study of API learning obstacles"** (Empirical Software Engineering 16:703–732, 2011; IEEE Software 2009 is the shorter version). Exploratory survey: 1,000 Redmond Microsoft developers invited, **80 (8%) answered**, 165 free-form obstacles. Respondents per obstacle category: **learning resources (documentation, code examples) 50**, API structure 36, developer background 17, technical environment 15, process 13. Then 28 interviews, and a follow-up survey of 1,936 developers with **334 (17.3%) answering**. The authors conclude that "surveyed developers also report documentation-related obstacles as the most severe." Five factors for documentation design: documentation of intent, code examples, matching APIs with scenarios (cookbooks), penetrability of the API, and format/presentation. One participant quote: "if the documentation for an API is good, it solves 99% of your problems." — [Robillard & DeLine 2011 PDF](https://www.cs.mcgill.ca/~martin/papers/ese2011.pdf)
- **Uddin & Robillard, "How API Documentation Fails"** (IEEE Software 32(4), 2015).
  - Survey 1: 698 IBM Canada employees invited, **69 responded (9.9%)**, giving 179 examples of good or bad docs for 131 documentation units.
  - Survey 2: **254 responses (23.8%)**, 80% developers.
  - Ten problems were studied. Respondents rated six of them "Blocker" at least once ("I could not get past it and picked another API"): incompleteness, ambiguity, obsoleteness, incorrectness, inconsistency, unexplained examples.
  - Ambiguity: "Frequently" observed by 26.5% (excluding "No Opinion") and chosen as top priority 51.9% of the time.
  - Incorrect content was rarely seen but judged "Severe" or "Blocker" when it was.
  - The authors' wording: "the respondents cared most about quality content … prioritized addressing five content-related problems over any presentation problem."
  - Fragmentation (clicking through many pages) and tangled information were the presentation problems.
  - No significant difference between novices and experienced respondents.
  — [Uddin & Robillard 2015 PDF](https://www.cs.mcgill.ca/~martin/papers/ieeesw2015.pdf)
- **Aghajani et al., "Software Documentation Issues Unveiled"** (ICSE 2019). Mined and manually analysed **878 documentation-related artifacts** from mailing lists, Stack Overflow, issues and pull requests, giving a taxonomy of **162 types of issues**.
  - Content issues: completeness 268 (incompleteness mostly hits API references and code comments); up-to-dateness 190 ("39% of issues related to documentation content"); correctness.
  - **Code examples were 50% of correctness issues** where a documentation type was labelled; installation guides were next at 20%.
  - The authors recommend testing code examples like production code, plus regression tests for snippets.
  — [Aghajani et al. 2019 PDF](https://emadpres.github.io/pdfs/icse2019.pdf)
- **Aghajani et al., "Software Documentation: The Practitioners' Perspective"** (ICSE 2020, DOI 10.1145/3377811.3380405). Two surveys of **146 practitioners**, 125 from one multinational. Survey 1 asked which issues matter most and how practitioners solve them; survey 2 asked which documentation types matter for which tasks. — [ICSE 2020 listing](https://2020.icse-conferences.org/details/icse-2020-papers/28/Software-Documentation-The-Practitioners-Perspective); [researchr (archived)](https://web.archive.org/web/20251013080955/https://researchr.org/publication/Aghajani0LMBLS20). The per-issue percentages were not retrieved (the PDF at usi.ch returned 403).
- Self-report vs observation: Meng et al. cite Lethbridge, Singer & Forward (2003). Engineers there claimed about 40% of their time went to reading documentation, but only 3% of logged events were documentation-related. — cited in [Meng et al. 2019](https://mangold-international.com/resources/meng-et-al-how-developers-use-api-documentation-2019.pdf) **[second-hand for the Lethbridge figure]**

### Inferences
- Content correctness and completeness beat navigation polish. For a PDF library this argues for testing every code example in CI, version-pinning pages, and documenting intent and edge cases, not only signatures.
- Code examples are both the most-wanted content and the most often wrong, so testing snippets is the highest-leverage practice the evidence supports.
- Self-reported docs use can overstate real use by an order of magnitude. Prefer analytics and observation to surveys when evaluating the redesign.

### Gaps
- The ranked results of the ICSE 2020 paper (e.g., the share citing outdated docs or missing examples) were not retrieved.
- Nasehi et al. (2012) on what makes a good Stack Overflow code example: not checked within the budget.

## 3. How developers actually read docs (observation and eye-tracking)

### Takeaway
The best observational evidence (Meng, Steinhardt & Schubert, 11 developers, eye tracking) finds:
- developers spend about half their task time in the docs;
- the API reference and code-example pages draw the most time;
- developers split into "opportunistic" (code first) and "systematic" (concepts first) styles;
- everyone starts from example code;
- search is expected.

The same study warns that a doc-type navigation (Concepts / Samples / Reference / Recipes) confused participants, and recommends organising by API functionality. That partly cuts against Diátaxis-style top-level splits.

### Cited Findings
- **Meng, Steinhardt & Schubert, "How Developers Use API Documentation: An Observation Study"** (Communication Design Quarterly, ACM SIGDOC, online first January 2019, DOI 10.1145/3274995.3274999).
  - Method: 11 participants from three organisations; experience 1–25 years (mean 9); a REST shipping API (shipcloud); screencasts, an SMI REDm eye tracker and questionnaires; sessions of 40–70 minutes; the authors say 11 participants are "too small to warrant statistical" testing.
  - Time: mean time per task 695 s (SD 282). Participants used the documentation **about 49% of the time (min 31%, max 68%)**, with all but two between 41% and 56%.
  - Content: the **API reference was the most-used category**, then Recipes. Samples and Recipes (code examples) together were "head to head with the API reference", each **about 21% of the total time**.
  - Concepts page: 5 participants spent 0–5.6% of their docs time on it; the other 6 spent 12.5–42.9%. Use did not track experience.
  - **Opportunistic** developers started the first task from example code in the docs and modified it, jumped from page to page, wanted "fast and direct access", and did not read larger sections. **Systematic** developers read concepts first, but "both opportunistic and systematic developers use code examples as starting" points.
  - Domain knowledge (e-commerce), not general experience, separated fast from slow performers.
  - Barriers: (1) the high-level split into "Concepts", "Samples", "API reference" and so on. "Participants sometimes found it difficult to decide which content category to select", the "Samples" vs "Recipes" distinction was unclear, and the label "API reference" suggested the section held the entire documentation. (2) **No search**: participants fell back on in-page search, which failed because content was spread over pages.
  - Recommendations: "API documentation should be structured according to categories that reflect the functionality or content domain of the API rather than using categories that signal the type of information provided"; provide a powerful search; enable selective access to code; support both strategies.
  — [Meng et al. 2019 PDF](https://mangold-international.com/resources/meng-et-al-how-developers-use-api-documentation-2019.pdf)
- Background: Clarke's (2007) personas (systematic, opportunistic, pragmatic), as summarised by Meng et al. — [same PDF](https://mangold-international.com/resources/meng-et-al-how-developers-use-api-documentation-2019.pdf). Meng et al. 2018 (Journal of Technical Writing and Communication, "Application Programming Interface Documentation: What Do Software Developers Want?") is the survey and interview companion; not read here — [SAGE DOI 10.1177/0047281617721853](https://www.sciencegate.app/document/10.1177/0047281617721853).
- Robillard & DeLine (2011) also recommend "matching APIs with scenarios" (cookbooks) and code examples as key design factors. — [PDF](https://www.cs.mcgill.ca/~martin/papers/ese2011.pdf)

### Inferences
- Design for code-first readers: a runnable example near the top of each page, and copyable, complete snippets.
- Search is not optional. A Rust core, a TS/WASM SDK and a CLI spread content across many pages, which is exactly the case where in-page search failed.
- Diátaxis/Divio has **no direct empirical validation in the sources found**. The one observational study that tested a doc-type split saw confusion at the top level. A compromise the evidence supports: top-level navigation by task or feature (open, extract text, build, render, sign…), with tutorial, how-to, reference and explanation kept as page types within each area and clearly labelled.

### Gaps
- No large-sample data was found on search vs sidebar share, the percentage who read top to bottom, or time on page, beyond the Meng study (n=11).
- I found no peer-reviewed evaluation of Diátaxis.

## 4. Measured business effects of better docs

### Takeaway
No credible, primary-source quantitative case study was verified within this budget. Claims about Stripe, Twilio or "time to hello world" deflection circulate mainly in vendor marketing.

### Cited Findings
- Postman 2025 says revenue-generating API organisations commonly use "developer-focused documentation", which "makes adoption faster and more successful". This is an analyst statement with no figure in the readable text. — [Postman 2025](https://www.postman.com/state-of-api/2025/)
- Uddin & Robillard (2015): six documentation problems were "blockers" that made respondents pick another API. This is the closest academic evidence that docs affect tool choice. — [PDF](https://www.cs.mcgill.ca/~martin/papers/ieeesw2015.pdf)
- GitHub 2017: 64% rate a licence as very important when deciding to use an open-source project. — [opensourcesurvey.org](https://opensourcesurvey.org/2017/)

### Inferences
- For a free open-source library, measurable proxies are time to first successful run (from install to output PDF), search success, and issues labelled as docs. Track them rather than relying on industry claims.

### Gaps
- No verified numbers on support-ticket deflection, activation or time-to-first-call from Stripe, Twilio or similar. Any such figure should be treated as marketing until traced to a primary source.

## 5. AI and docs (2024–2026), including llms.txt

### Takeaway
AI assistants are now a main way developers search for answers, but trust is low. llms.txt is spreading among top sites while AI crawlers rarely fetch it. All llms.txt data comes from SEO or marketing vendors, not peer-reviewed research.

### Cited Findings
- SO 2025: 84% use or plan to use AI tools; 46% distrust their accuracy; 66% cite "almost right" answers; AI tools rank first (52.5%) for learning to code for AI, ahead of docs (42.5%). — [SO 2025 AI](https://survey.stackoverflow.co/2025/ai); [SO 2025 Developers](https://survey.stackoverflow.co/2025/developers)
- SO 2024: 37% used "AI" as an online resource to learn to code, vs 83.9% for technical documentation. — [SO 2024](https://survey.stackoverflow.co/2024/developer-profile)
- Postman 2025: 70% aware of MCP, 10% using it regularly; 41% use AI to generate API docs; "AI agents rely on precise, machine-readable signals". — [Postman 2025](https://www.postman.com/state-of-api/2025/)
- llms.txt request rates, adoption and support, all **[second-hand / vendor research]**:
  - Ahrefs server logs from 137,000 domains: **97% of llms.txt files received zero requests in May 2026**. Originality.ai tracker: 4,088 llms.txt files (June 2025) → 36,120 (May 2026) across more than 3 million monitored sites. — via [PPC Land](https://ppc.land/llms-txt-adoption-rises-8-8x-but-97-of-files-get-zero-ai-requests/)
  - Rankability scan (September 2026): 8.3% of the top 10,000 domains and 9.3% of the top 1,000 serve a confirmed llms.txt or llms-full.txt. — [Rankability](https://www.rankability.com/llms-report/)
  - Otterly test: 84 llms.txt requests among more than 62,100 AI bot visits (about 0.1%). SE Ranking (about 300,000 domains): 10.13% adoption and no clear effect on AI citations. Google said in July 2025 it does not support llms.txt. — via [Optimizely](https://www.optimizely.com/field-notes/articles/does-llms-txt-work/)

### Inferences
- llms.txt is cheap and harmless, but crawler-side evidence of value is weak. The real AI pathway is likely coding agents fetching Markdown pages, and docs shipped inside packages, which Lamina already does. Clean per-page Markdown and accurate, versioned content matter more than the index file.
- With "almost right" AI answers common, error-code pages, exact signatures and tested examples are what users fall back on, and these give agents a ground truth.

### Gaps
- No independent or peer-reviewed study found on what share of developers use AI chat *instead of* docs. SO's questions measure use alongside docs, not substitution.
- No measured data on agent traffic to developer docs sites (e.g., share of docs page views from agents) from a neutral source.
- 2023–2026 academic studies on LLMs and API documentation were not reviewed within the budget.
