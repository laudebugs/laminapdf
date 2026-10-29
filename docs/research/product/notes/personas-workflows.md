# User personas, PDF task frequency and workflows (as of Oct 2026)

Scope: evidence for functional requirements for three personas (academic, business/legal, casual mobile) and data on how often PDF tasks happen, to inform lamina's UI/UX plan. Research done with web search on 2026-10-29. Two sites could not be fetched from this session (webaim.org and business.adobe.com are outside the session's allowed network domains), so figures from them come from search snippets and secondary sources and should be re-checked against the originals.

## Q1. Quantitative data on PDF task frequency (validate the 70 / 20 / 8 / 2 hypothesis)

### Takeaway
I found **no public dataset that splits PDF activity by task** (view vs annotate vs organise vs convert/OCR): not from Adobe, Smallpdf or iLovePDF, and not in academic work. The hypothesis cannot be confirmed or refuted with numbers. The indirect evidence says viewing dominates. It also says the web-tool market (1B+ users) is built on compress, merge, split and convert, so the "~8% organise, ~2% convert" shares are probably **too low for a web app's entry traffic**, even if they hold for time spent in a desktop reader.

### Cited Findings
- I found no 2024–2026 Adobe survey that directly measures PDF usage by task among knowledge workers. Most Adobe figures are vendor-commissioned (Forrester TEI, Pfeiffer benchmark, "State of AI in Documents", 400+ professionals) — [Adobe State of AI in Documents](https://business.adobe.com/assets/pdfs/resources/sdk/discover-how-ai-is-powering-document-productivity/state-of-ai-in-documents.pdf); [Forrester TEI of Acrobat Studio](https://tei.forrester.com/go/adobe/acrobatstudio/); [Pfeiffer AI Assistant benchmark](https://business.adobe.com/content/dam/dx/us/en/resources/sdk/acrobat-ai-assistant-pfeiffer-report/pfeiffer-ai-assistant-benchmark-report.pdf)
- Adobe survey of U.S. financial professionals: 98% "regularly edit, sign, convert, combine, or collaborate on PDF documents". A later summary attributes "61% regularly edit PDFs" to a 2025 Adobe survey. These are search snippets; the page could not be fetched — [Adobe blog: more financial professionals trust Acrobat](https://business.adobe.com/blog/more-financial-professionals-trust-acrobat)
- Adobe marketing cites 8.2 hours/week spent searching for, re-creating or fixing documents, from an **APQC 2021** report (older than 2022; flagged) — via search snippets of [Adobe resources](https://www.adobe.com/acrobat/resources/adobe-ai-assistant-time-saving-analysis.html)
- Adobe: over **1 billion files read in Liquid Mode** (its mobile reflow view) as of Feb 2024 — [Adobe blog, 2024-02-08](https://blog.adobe.com/en/publish/2024/02/08/adobe-liquid-mode-recognized-edison-awards-finalist)
- iLovePDF (15-year anniversary post): 860M+ unique visitors a year, ~20M users a day, among the top-25 most visited sites. The 2010 version did only merge and split; tools were added "because users kept asking for the next step in the workflow". No per-tool ranking is published — [iLovePDF blog](https://www.ilovepdf.com/blog/ilovepdf-15-years-anniversary-milestones)
- Smallpdf claims over 1 billion users (self-reported; another source says 1.7B since 2013, so the figures are inconsistent). There is no per-tool usage data. Third-party listings and reviews keep naming Compress, Merge and PDF↔Word conversion as the main tools — [Smallpdf About](https://smallpdf.com/de/about); [Capterra Smallpdf reviews](https://www.capterra.com/p/172606/Smallpdf/reviews/?page=4)
- Reviewers most often praise iLovePDF for merging (sentiment, not usage data) — [Capterra iLovePDF](https://www.capterra.co.uk/reviews/173963/ilovepdf)

### Inferences
- **Estimate (not data):** in a *reader/editor* (desktop or a web app opened on a file), share of sessions is probably still dominated by viewing/search (60–75%). But the *web-tool* market is task-driven: users arrive to compress, merge, split, convert or sign, and then leave. If lamina's web app wants that traffic, page organisation plus compression/conversion could be 20–40% of **entry intents**. Use something like 70 / 20 / 8 / 2 for time-in-app and treat organise/convert as a much larger share of **first-visit intents**. Validate both with lamina's own opt-in analytics.
- Liquid Mode's billion reflowed files suggest that mobile reading of fixed-layout PDFs is a large, real use case (the next paragraph covers its limits).
- The iLovePDF note ("next step in the workflow") supports building chained workflows (merge → compress → sign → share) rather than isolated tools.

### Gaps
- No public task-frequency split from Adobe, Smallpdf, iLovePDF, Foxit or PDF Expert, and no HCI log study of PDF task mix (2022–2026) was found.
- Similarweb-type per-tool page traffic for smallpdf.com and ilovepdf.com was not available. It is the best proxy if someone has access.
- The original wording and sample size of the Adobe financial-professional survey could not be verified.

## Q2. Persona needs: jobs-to-be-done, must-haves, deal-breakers, devices

### Takeaway
**Academics** need annotation that round-trips with their citation manager (standard PDF annotations, exportable highlights), split view, in-place previews of citations and definitions, and dark mode. Their reading is still largely print-or-tablet for close reading. **Business/legal users** have two hard deal-breakers: redaction that leaves text recoverable (an incident as recent as Dec 2025), and signatures without legal standing (eIDAS QES/AdES via PAdES). **Casual mobile users** need reflowed reading, scan-to-PDF, quick sign and share, and files that fit email limits.

### Cited Findings

**(a) Academic / researcher**
- Survey (n=416, published 2015, older — flagged): PDF is the dominant format for reading and saving scholarly work. E-readers, tablets and phones were not popular for working with the literature, and most participants preferred to print PDFs and annotate by hand — [Moed et al., Bar-Ilan CRIS](https://cris.biu.ac.il/en/publications/accessing-reading-and-interacting-with-scientific-literature-as-a/)
- Rebus Community scholarly-reading survey (~104 respondents, undated): print first for close reading, digital during writing. Syncing across devices and annotation tools rated important — [Rebus press](https://press.rebus.community/scholarlyreading/?p=34)
- ScholarPhi (CHI 2021): position-sensitive tooltips for definitions of terms and symbols, equation diagrams and an auto-glossary. A usability study found it helped researchers at all experience levels, and they wanted it for everyday reading. The formative study had n=9. It was evaluated in the lab with hand-edited definitions — [arXiv 2009.14237](https://arxiv.org/abs/2009.14237v2); [NSF PAR copy](https://par.nsf.gov/servlets/purl/10302719)
- Semantic Reader (CACM 2023): in-context "Paper Cards" pop up when you click an inline citation, "reducing the interaction cost of jumping back and forth" to the reference list. The pipeline moved from LaTeX to PDF-only, expanding coverage from ~500k to ~5M papers. Open libraries: PaperMage, PaperCraft — [CACM: The Semantic Reader Project](https://cacm.acm.org/research/the-semantic-reader-project); [Semantic Scholar release notes](https://semanticscholar.org/product/release-notes)
- Readwise imports PDF highlights only if they are standard PDF annotations. MarginNote, GoodNotes and Notability highlights cannot be extracted. Highlights spanning pages split into two. Scanned/OCR PDFs import less reliably — [Readwise docs: Import from PDF](https://docs.readwise.io/readwise/docs/importing-highlights/pdf)

**(b) Business / legal professional**
- *Redaction failure, Manafort (Jan 2019):* the defence filing's "redactions" were black boxes drawn over live text, so reporters copied and pasted the hidden passages, revealing information that had been confidential. Analysis says Acrobat Pro markup tools were used instead of the Redaction tool — [ABA Journal](https://www.abajournal.com/news/article/paul-manaforts-attorneys-failed-at-redacting-learn-how-to-do-it-right); [Above the Law](https://abovethelaw.com/2019/01/paul-manaforts-lawyers-learn-that-obvious-thing-you-learned-as-a-summer-associate/); [Foxit developers blog](https://developers.foxit.com/blog/bad-redaction-manafort-caught/)
- *Redaction failure, DOJ Epstein files (Dec 2025):* some files released under the Epstein Files Transparency Act had redactions defeated by copy and paste, because a black image was placed over text that stayed in the file. Snopes confirmed at least one document (not all). Reports say DOJ pulled files after victim information was exposed. Counts on social media (e.g. "600+ mentions") are unreliable — [Mediaite](https://www.mediaite.com/media/news/doj-redactions-of-some-epstein-files-easily-revealed-with-copy-and-paste); [GV Wire/AP-style report 2025-12-24](https://gvwire.com/2025/12/24/redacted-material-in-some-epstein-files-is-easily-recovered/); [Decrypt](https://decrypt.co/353341/tiktok-users-claim-theyve-unredacted-the-epstein-files)
- The ABA Judges' Journal (2019) catalogues other "embarrassing redaction failures", including black-coloured text and highlight-as-redaction that can be undone by changing the colour — [ABA Judges' Journal](https://www.americanbar.org/groups/judicial/publications/judges_journal/2019/spring/embarrassing-redaction-failures/)
- *eIDAS (Reg. (EU) 910/2014), Art. 25:* (1) an e-signature cannot be denied legal effect solely for being electronic; (2) a **qualified** electronic signature (QES) has the effect of a handwritten signature; (3) a QES from one Member State is recognised in all of them. QES requires a qualified certificate plus a qualified signature creation device (QSCD). PAdES (alongside XAdES and CAdES) is the PDF format for advanced/qualified signatures. PAdES is a *format*, and its legal level depends on the certificate and device — [Wikipedia: Qualified electronic signature](https://en.wikipedia.org/wiki/Qualified_electronic_signature); [Wikipedia: Advanced electronic signature](https://en.wikipedia.org/wiki/Advanced_electronic_signature); [Adobe Sign: EU legality](https://helpx.adobe.com/ca/sign/using/legality-european-union.html). Secondary sources; check against the Official Journal text and ETSI EN 319 142 (PAdES).
- *Email limits (compression):* Gmail caps outgoing messages at 25 MB total (50 MB receiving). Outlook.com/internet accounts are reported at 20 MB, Exchange defaults at 10 MB, and sources conflict. Base64 encoding inflates a 20 MB file to ~27 MB — [IONOS digital guide](https://www.ionos.com/digitalguide/e-mail/technical-matters/gmail-attachment-size-limit.md); [Palisade](https://www.palisade.email/learning/max-size-for-email-attachment)
- Financial professionals: 98% regularly edit, sign, convert, combine or collaborate on PDFs (Adobe, see Q1) — [Adobe blog](https://business.adobe.com/blog/more-financial-professionals-trust-acrobat)

**(c) Casual mobile user**
- Adobe Liquid Mode (2020–): reflows text, tables, bullets and headers for small screens. It has read 1B+ files (Feb 2024). It processes in Adobe's cloud, which raised privacy concerns, and does not apply to files over 200 pages — [Adobe blog 2024](https://blog.adobe.com/en/publish/2024/02/08/adobe-liquid-mode-recognized-edison-awards-finalist); [The Register 2020](https://www.theregister.com/2020/09/24/adobe_liquid_mode_pdf_mobile/); [Adobe Help: Liquid Mode](https://helpx.adobe.com/gr_en/acrobat/mobile/view-manage-files/liquid-mode.html)
- NN/g, "PDF: Still Unfit for Human Consumption, 20 Years Later": PDF usability problems now extend to mobile. Put essential content on web pages and keep PDFs for printing — [NN/g](https://www.nngroup.com/articles/pdf-unfit-for-human-consumption/) (via [DesignHammer summary](https://designhammer.com/blog/pdfs-should-have-been-cancelled-decades-ago-according-nielsen-norman-group); NN/g article is from ~2020, so flagged as older)

### Inferences (recommendations, labelled as opinion)
- **Academic must-haves:** standard `/Annot` highlights, notes and ink written into the file (so Zotero, Readwise and others can read them); export of highlights to Markdown/CSV/BibTeX-linked notes; vertical/horizontal split view with annotation tools working in both panes (a Zotero beta complaint, see Q4); hover/click previews of citations, figures and internal links (Semantic Reader and ScholarPhi show this reduces flipping); dark/sepia mode that recolours page content; tablet + pen support. **Deal-breakers:** proprietary annotation storage, and annotations that do not sync. **Devices:** laptop/desktop plus iPad/Android tablet; many still print.
- **Business/legal must-haves:** AcroForm fill with tab order and validation, plus "fill flat form" (free text placed on non-interactive forms); true redaction that removes the text, images and vector content underneath, sanitises metadata, hidden layers and incremental-update history, and verifies after save (e.g. no text extractable under the redaction rectangles). Also: Bates numbering and header/footer stamps, merge/split/reorder, compression to a target size (e.g. "under 20 MB"), visual and text compare, PDF/A conversion and validation, and two clearly separated signing paths: simple "e-sign" (drawn or typed image) and PAdES digital signatures (B-B/B-T/B-LT/B-LTA), with QES through a QSCD or remote QTSP. **Deal-breakers:** recoverable redaction, signatures that do not validate in Acrobat, broken forms. **Devices:** Windows desktop first, then web.
- **Casual mobile must-haves:** fast open from the share sheet, reflow/"reading mode", find, camera scan with edge detection and OCR, sign with a finger (place, resize, date), share back. All local, which is a privacy advantage over cloud reflow. **Deal-breakers:** sign-up walls, uploading to a server, tiny unzoomable text. **Devices:** phone (iOS/Android), sometimes a web browser on the phone.
- The two redaction incidents, six years apart, show that the UI must make it **impossible to confuse a black rectangle annotation with a redaction**: separate tool, mark → review → apply flow, and a post-apply verification report.

### Gaps
- No 2022–2026 quantitative survey of researchers' device split (tablet vs laptop vs print) was found; the best data is from 2015.
- No published data on Bates numbering, PDF/A or compare usage frequency among legal users.
- No survey data on how often casual users sign leases or scan on phones (Adobe Scan download counts not found).
- eIDAS 2.0 (Reg. (EU) 2024/1183, the EU Digital Identity Wallet) effects on signing in apps were not researched here.

## Q3. Accessibility needs and mobile reading of fixed-layout PDFs

### Takeaway
PDFs are seen as the least accessible common document format by screen-reader users (75% said PDFs are likely to pose significant issues, WebAIM 2019). Reflow is the main answer for low vision, dyslexia and small screens, but it depends on tags or AI structure inference.

### Cited Findings
- WebAIM Screen Reader Survey #8 (Aug–Sep 2019; older, flagged): 75.1% said PDF documents are very or somewhat likely to pose significant accessibility issues, vs 30.9% for Word documents — [WebAIM survey 8 results](https://webaim.org/blog/screen-reader-survey8-results/)
- WebAIM Survey #10 (Dec 2023–Jan 2024, 1,539 respondents) asked whether screen-reader users use PDF bookmarks as their main navigation. The result was not in the snippets, and webaim.org could not be fetched from this session — [WebAIM survey 10 results](https://webaim.org/blog/screen-reader-user-survey-10-results/); [Adrian Roselli takeaways](https://adrianroselli.com/2024/02/my-webaim-10th-sr-user-survey-takeaways.html)
- A 2021 secondary source: 68.9% of screen-reader users find Word the most accessible document format — via [WebAIM search results](https://webaim.org/blog/screen-reader-survey-results/) (unverified)
- Comparison of PDF viewer apps with screen readers: viewer choice affects access as much as tagging does — [Information Access Group](https://informationaccessgroup.com/comparing-how-well-pdf-viewer-applications-work-with-screen-readers/)
- Adobe positions Liquid Mode as an accessibility/readability feature (adjustable font size and spacing in reflowed view) — [Adobe blog, 2020-12](https://blog.adobe.com/en/publish/2020/12/10/adobe-delivers-enhanced-pdf-reading-experience-for-accessibility); [Readability Matters](https://readabilitymatters.org/articles/time-recognizes-adobe-liquid-mode-as-best-invention-of-2023)

### Inferences
- Requirements (recommendation): expose the tag tree/structure to platform accessibility APIs (web: ARIA from the text layer; Tauri: the webview's a11y tree); navigation by bookmarks (outline), headings and links; full keyboard operation of every tool; high-contrast and inverted page colours; reflow mode built from tags, falling back to heuristic layout analysis, with user font, size, spacing and dyslexia-friendly settings; text-to-speech in reading order. All of it local, unlike Liquid Mode's cloud processing.

### Gaps
- No recent (2022–2026) quantitative data on mobile reading of fixed-layout PDFs (zoom/pan behaviour, abandonment) was found.
- No data was found on dyslexic or low-vision users' PDF use.
- WebAIM #10's PDF figures need a direct fetch (the domain was blocked here).

## Q4. Competitors for the academic persona: Zotero reader, Apple Books, Kindle, Readwise, Paperpile

### Takeaway
Zotero 7's built-in reader is the benchmark: it annotates PDF, EPUB and snapshots, has split view and dark mode, and turns annotations into notes. It stores annotations in its database rather than the file, so interoperability depends on export. Readwise requires spec-compliant annotations. Apple Books and Kindle are weak for academic PDF work. Paperpile is Google Drive-centred, with iPad annotation.

### Cited Findings
- Zotero 7 (released Aug 2024): redesigned reader. EPUBs open and annotate like PDFs. Split view horizontally or vertically. Underline and text annotations added. Dark mode for page content, which can be switched off. Webpage snapshots can be annotated — [Zotero blog: Zotero 7](https://www.zotero.org/blog/zotero-7/); [TUHH](https://www.tub.tuhh.de/en/2024/08/12/zotero-7-is-released/)
- Zotero stores annotations in its database, not in the PDF. You can export a copy with annotations embedded. Annotations from external readers show read-only until imported. Annotations can be extracted into notes — [Uppsala UB libguide](https://libguides-en.ub.uu.se/zotero/PDFReader); [Polytechnique Montréal guide](https://libguides.biblio.polymtl.ca/zotero/pdf-reader)
- A beta-era forum report says annotation tools did not work in the second split pane (may be fixed) — [Zotero forums](https://forums.zotero.org/discussion/comment/450564/)
- Zotero for iOS added EPUB and snapshot annotation, and PDF metadata retrieval — [Zotero blog](https://www.zotero.org/blog/?p=2663)
- Readwise Reader: highlights on a PDF's text view and on its PDF view do not sync with each other. PDF highlight import needs standard annotations — [Readwise changelog](https://readwise.io/changelog/pdfs); [Readwise docs](https://docs.readwise.io/readwise/docs/importing-highlights/pdf)
- Apple Books: Readwise says Apple Books does not create spec-compliant PDF highlights. A developer blog says it writes standard annotations into the file. The two sources conflict — [Readwise help](https://help.readwise.io/article/50-can-i-import-highlights-from-pdfs-read-in-apple-books); contradicted by [DEV Community post](https://dev.to/eristoddle/apple-books-hides-your-pdf-highlights-my-obsidian-plugin-now-digs-them-out-lio)
- Kindle: historically no annotation on PDFs; newer devices/updates report text selection, highlights and notes on PDFs. Readwise's Kindle sync does not clearly cover Kindle PDFs; third-party tools embed Kindle PDF highlights back into the file — [UPDF knowledge base](https://updf.com/knowledge/can-you-annotate-on-a-kindle/); [Zotero forums: Kindle Scribe](https://forums.zotero.org/discussion/comment/448594)
- Paperpile: PDFs are stored and synced in Google Drive (Settings › Data and Files › Sync to Google Drive). The iPad app supports Apple Pencil annotation. Some users report annotation glitches — [G2 Paperpile reviews](https://www.g2.com/products/paperpile/reviews); [UCreative libanswers](https://ucreative.libanswers.com/faq/191557)

### Inferences
- lamina's opening (opinion): be the reader that writes **standard, interoperable annotations into the file** (which Readwise, Zotero import and Acrobat can all read), with Zotero-class features (split view, dark mode, notes export) plus Semantic-Reader-style citation and figure previews, working offline and on any platform. Zotero integration (open from the Zotero library, import/export annotations via the Zotero API or a local connector) is likely more valuable than competing with Zotero.

### Gaps
- Paperpile's official docs on annotation storage (in-file vs database) were not found.
- Kindle's current PDF annotation export behaviour was not confirmed from Amazon sources.
- Mendeley's current reader status (Mendeley Reference Manager) was not researched.
