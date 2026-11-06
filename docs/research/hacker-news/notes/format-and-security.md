# The PDF format, security, trust and the ecosystem on Hacker News, Nov 2016 – Nov 2026

*Part of [What PDF users ask for: ten years of Hacker News](../pdf-users-on-hacker-news.md). How the data was collected: [method](method.md). The work it leads to: the [trust and proof plan](../../../plans/trust-and-proof.md).*

Area: opinions of PDF as a format, security, privacy, redaction and forensics, authenticity and signatures, accessibility, archiving, licensing and fonts. Counts come from `tools/hn-research/counts/format-and-security/format_security_counts.py` (method in §7). Each count gives the regex hits, the hits per period (P1 = 2016–2019, P2 = 2020–2022, P3 = 2023–2026), the rate per 1,000 PDF comments in each period, and how many of 12 random hits were on topic when I read them. The ranking uses an estimate: hits × the on-topic share, rounded to the nearest 10. Themes overlap, so the estimates do not add up to a total.

## 1. What users want

Ranked by the estimated number of on-topic comments.

**1. Safe opening of untrusted PDFs (about 270).** People want to open a PDF from a stranger without running its code or trusting a C parser. Measured by `sandbox_isolation` (sandbox, Qubes, Dangerzone, VM, memory-safe, near a PDF name): 259 hits, 44 / 96 / 119 (4.1 / 5.6 / 4.4 per 1,000), 9 of 12 on topic. Plus `want_safe_static` (disable or strip JavaScript, sanitise, flatten to images, a static or locked-down viewer): 131 hits, 14 / 63 / 54, 7 of 12 on topic.

> "Seems like there would be market demand for a super locked down PDF viewer that basically ignores all the silly extensions/additions that Adobe has added to the format over the decades." — 2022-07-06, [32004828](https://news.ycombinator.com/item?id=32004828)

> "IMHO a PDF should be a static document, hence, something you can trust without worrying." — 2024-04-13, [40023071](https://news.ycombinator.com/item?id=40023071)

> "It would be better to strip active content like javascript and actions, without flattening the PDF and losing all the text data" — 2026-01-22, [46722698](https://news.ycombinator.com/item?id=46722698)

**2. A file that looks the same everywhere and stays put (about 210).** The praise for PDF is that it is a finished, self-contained copy that prints and displays as the author made it, with no trackers and no link rot. Measured by `prefer_pdf_over_web` (131 hits, 28 / 56 / 47, 8 of 12), `praise_self_contained` (137 hits, 24 / 53 / 60, 6 of 12) and `praise_fixed_layout` (96 hits, 18 / 39 / 39, 6 of 12).

> "When I want a PDF, it's because I want a format that I know is always going to look the same." — 2019-11-21, [21591588](https://news.ycombinator.com/item?id=21591588)

> "with PDF, I know for a fact that no web requests are made in the background. That means no fingerprinting, no analytics etc." — 2021-07-19, [27881185](https://news.ycombinator.com/item?id=27881185)

> "PDF is the only file type I can export to and be confident it will render correctly on the recipient's machine." — 2023-02-08, [34712185](https://news.ycombinator.com/item?id=34712185)

**3. PDF/A and files that last (about 180).** People name PDF/A as the subset that keeps PDF's good parts for archives and drops the risky ones, and they complain that free tools rarely produce it. Measured by `pdfa_archival` (PDF/A, veraPDF, ISO 19005): 194 hits, 45 / 72 / 77 (4.2 / 4.2 / 2.9), 11 of 12 on topic. A looser `longevity_archiving` theme (archival, "in 20 years", bit rot) has 269 hits but only 4 of 12 on topic.

> "I have policy and legal documents from 2005 in PDF/A that can be rendered identically in 2019, and likely in 2105." — 2019-05-03, [19820915](https://news.ycombinator.com/item?id=19820915)

> "A PDF generator library that only generated guarenteed PDF/A compliant PDFs would actually be really good selling point for a new PDF library." — 2022-11-18, [33654190](https://news.ycombinator.com/item?id=33654190)

> "only commercial pdf libraries support PDF/A." — 2025-08-04, [44782733](https://news.ycombinator.com/item?id=44782733)

**4. Processing on my own machine, not someone's server (about 150).** People refuse to upload tax forms, contracts and medical papers to online PDF sites, and in P3 "client-side" tools became a genre. Measured by `privacy_local_processing` (client-side, in-browser, no upload, uploading sensitive files to random sites): 361 hits, 55 / 83 / 223 (5.1 / 4.8 / 8.3), 5 of 12 on topic.

> "to avoid my partner uploading sensitive documents to random PDF manipulation websites." — 2023-12-25, [38765763](https://news.ycombinator.com/item?id=38765763)

> "Seems like Clientside PDF editors are the new "hello world" app these days." — 2026-01-19, [46675926](https://news.ycombinator.com/item?id=46675926)

> "Whenever you need to split, merge, compress, or sign a confidential document (tax forms, legal contracts, medical paperwork) using online tools, your files are transmitted to a third-party server." — 2026-10-26, [50238740](https://news.ycombinator.com/item?id=50238740)

**5. A permissively licensed library (about 150).** Developers want a PDF library they can ship in a closed product, and many answers in "which library?" threads turn on the licence. Measured by `licensing_pdf_libs` (a licence word within 150 characters of a PDF library name): 219 hits, 54 / 50 / 115 (5.0 / 2.9 / 4.3), 8 of 12 on topic. The AGPL complaints in §2 are a subset of these hits.

> "i need tools that work with my MIT and GPL released code." — 2024-01-18, [39037654](https://news.ycombinator.com/item?id=39037654)

> "Unfortunately the PDF generation ecosystem seems to be full of libraries that aren't permissively licensed" — 2024-01-07, [38905231](https://news.ycombinator.com/item?id=38905231)

> "couldn't find a permissively-licensed option that was both fast and handled edge cases well." — 2026-02-24, [47137423](https://news.ycombinator.com/item?id=47137423)

**6. Signatures that prove a document is genuine (about 140).** People want banks, landlords and agencies to sign PDFs, and they want a check that works without the issuer's website. Measured by `digital_signatures` (digital or cryptographic signing, PAdES, LTV, PKI, eIDAS, signed PDFs): 242 hits, 36 / 113 / 93 (3.3 / 6.5 / 3.5), 7 of 12 on topic.

> "They actively coax you into receiving your bank account statements as PDF, but I haven't found ANY bank which signs the PDFs" — 2022-12-19, [34051866](https://news.ycombinator.com/item?id=34051866)

> "they really need to provide a way to verify a document without the site being up and intact." — 2025-09-26, [45381358](https://news.ycombinator.com/item?id=45381358)

> "Signing PDFs digitally is kind of a pain, though. There are several different standards, compatible with different readers, and you need to pay up to get a trusted certificate" — 2025-09-26, [45384252](https://news.ycombinator.com/item?id=45384252)

**7. Redaction that really removes the text (about 100).** After each leak, people ask for a free tool that deletes what sits under the black box and warns users who only draw one. Measured by `want_real_redaction_tool` (redact near tool, feature, properly, real, permanent): 110 hits, 16 / 24 / 70 (1.5 / 1.4 / 2.6), 7 of 8 on topic.

> "Is there a good free tool to properly redact PDFs?" — 2025-12-23, [46370144](https://news.ycombinator.com/item?id=46370144)

> "I've failed to find even a single option on linux that does real PDF redaction like adobe acrobat." — 2025-11-29, [46085321](https://news.ycombinator.com/item?id=46085321)

> "Every PDF editor ought to have a big flashing warning explaining to use a redaction tool instead, every time a user attempts to draw/move a black rectangle over text" — 2023-12-06, [38549034](https://news.ycombinator.com/item?id=38549034)

**8. Tagged, accessible output from generators (about 80).** Accessibility advocates ask every new PDF generator for tagged PDF and PDF/UA, because structure must be there from the start. Measured by the follow-up theme `tagged_pdf_ua_output` (tagged PDF, PDF/UA, structure tree, tagging): 135 hits, 24 / 39 / 72 (2.2 / 2.3 / 2.7), 5 of 8 on topic.

> "To everyone thinking about writing their own code to generate PDF, I'm begging you, please either implement tagged PDF support for accessibility, and test it with Adobe Reader and a screen reader" — 2022-05-05, [31272485](https://news.ycombinator.com/item?id=31272485)

> "In the future better accessibility for screen readers of text (paragraph breaks), images (alt text), tables (control of reading order), and math formulae." — 2024-06-07, [40613580](https://news.ycombinator.com/item?id=40613580)

> "You won’t be able to generate semantic HTML from inaccessible PDF, that needs to be there from day one." — 2025-11-11, [45887330](https://news.ycombinator.com/item?id=45887330)

**9. An HTML or EPUB version next to the PDF (about 80).** Readers on phones and screen readers want a reflowable copy, and they praise arXiv's HTML papers and Gov.uk's HTML-first policy. Measured by `prefer_html_over_pdf` (HTML or EPUB instead of PDF, "should be HTML", "hate PDFs"): 152 hits, 34 / 74 / 44 (3.1 / 4.3 / 1.6), 6 of 12 on topic.

> "almost every PDF I've interacted with in my adult life should have been a self-contained HTML page or file." — 2024-01-18, [39037135](https://news.ycombinator.com/item?id=39037135)

> "So, I think it makes more sense to do what arxiv is doing: providing a html version of articles on top of pdfs." — 2025-11-11, [45885629](https://news.ycombinator.com/item?id=45885629)

A smaller want: tools to look inside a file (qpdf's QDF mode, `mutool clean -d`, RUPS, PDF debuggers). `want_inspect_tools` has 38 hits, 5 / 11 / 22, and most hits I read were on topic. "There needs to be better tooling for inspecting PDF documents." — 2026-01-10, [46565528](https://news.ycombinator.com/item?id=46565528)

## 2. What they complain about most

**1. Readers get exploited (about 360).** The most repeated complaint is that opening a PDF can compromise a machine, and Acrobat carries most of the blame. Measured by `exploits_cves` (CVE, exploit, vulnerability, malware, zero-day, RCE, code execution, JBIG2, FORCEDENTRY, near a PDF name): 622 hits, 138 / 215 / 269 (12.7 / 12.5 / 10.0), 7 of 12 on topic.

> "The problem with PDF is that it's a bag of needles disguised as a piece of paper." — 2020-05-14, [23184955](https://news.ycombinator.com/item?id=23184955)

> "If you care about security, two things you don't want to install on your computer are Adobe Acrobat and Microsoft Office." — 2022-07-06, [32007816](https://news.ycombinator.com/item?id=32007816)

> "ghostscript will never be safe, ever." — 2023-07-14, [36718027](https://news.ycombinator.com/item?id=36718027)

**2. Unreadable on phones and small screens (about 270).** Fixed pages mean pinch, zoom and a diagonal scroll per column, so many call PDF a print format forced onto screens. Measured by `screens_reflow` (reflow, pinch-zoom, phone or e-reader near zoom or columns, "designed for print"): 466 hits, 103 / 222 / 141 (9.5 / 12.9 / 5.2), 7 of 12 on topic.

> "on a 16:9 screen, smartphone, or small tablet they're annoying, since you have to zoom in to read the text and then do a weird diagonal scroll between columns every page." — 2020-08-10, [24110359](https://news.ycombinator.com/item?id=24110359)

> "PDFs are paper-document oriented and don't display well for reading on anything other than a large size monitor in virtually every case I've seen." — 2018-07-16, [17542509](https://news.ycombinator.com/item?id=17542509)

> "Text not reflowing according to my screen/window size is most certainly a bug." — 2024-01-18, [39037455](https://news.ycombinator.com/item?id=39037455)

**3. Inaccessible to blind and low-vision readers (about 270).** Screen readers get broken characters, columns in the wrong order and unreadable tables and maths; tagging exists but almost nobody produces it. Measured by `accessibility_disability` (screen reader, tagged PDF, PDF/UA, WCAG, Section 508, alt text, blind users, accessible near disability words): 290 hits, 51 / 111 / 128 (4.7 / 6.4 / 4.8), 11 of 12 on topic.

> "Try any screen reader you like. If you are lucky you will get most of the text out with some characters randomly broken. Columns often don't read in the right order, tables and maths are both unreadable." — 2018-07-16, [17542657](https://news.ycombinator.com/item?id=17542657)

> "Tagged pdfs are still a thing for accessibility, to make pdfs work better with assistive technology. In practice, this is underutilized, because of the additional effort to use it properly." — 2020-09-23, [24569460](https://news.ycombinator.com/item?id=24569460)

> "It has a terrible impact on accessibility, as the whole text layer of the PDF is lost and it becomes unreadable for screen readers." — 2020-05-13, [23165696](https://news.ycombinator.com/item?id=23165696)

**4. Code inside documents (about 170).** Many commenters learn with alarm that PDFs carry JavaScript, XFA forms, Flash or 3D, and few accept form validation as the reason. Measured by `js_active_content` (JavaScript in a PDF, Acrobat's JS engine, XFA, /OpenAction, launch actions, embedded Flash or 3D): 347 hits, 78 / 113 / 156 (7.2 / 6.5 / 5.8), 6 of 12 on topic. The subset that calls it a risk (`js_as_risk`) has 59 hits, 8 of 12 on topic.

> "You can also (and really ought to) switch off the JavaScript engine completely in Acrobat reader. There isn't a legitimate reason to run Javascript during the viewing of a PDF." — 2020-08-10, [24110647](https://news.ycombinator.com/item?id=24110647)

> "I've seen javascript in PDFs be used for unintended exploits more often than every legitimate use combined. It's kind of like if JPEGs could run arbitrary code by design." — 2024-04-13, [40021483](https://news.ycombinator.com/item?id=40021483)

> "Then pdf came along and said: no this is too dangerous the only thing in a document should be layout information not arbitrary code." — 2025-01-13, [42680257](https://news.ycombinator.com/item?id=42680257)

**5. Anyone can fake a PDF or a signature (about 160).** People complain that businesses treat PDFs as unalterable while forging one is easy, and that most "signatures" are pasted images. Measured by `authenticity_forgery` (forgery, forged, tamper, doctored, fake or altered documents, "can't be edited"): 119 hits, 23 / 36 / 60, 8 of 12 on topic. Plus `image_signatures` (print-sign-scan, a PNG of a signature, FalsiScan): 94 hits, 11 / 62 / 21 (1.0 / 3.6 / 0.8), 10 of 12 on topic.

> "it boggles my mind how many people in the business world view PDF's as secure, unalterable documents." — 2018-02-23, [16448008](https://news.ycombinator.com/item?id=16448008)

> "Yup, I just "sign" in Acrobat Reader using a signature image I scanned a long time ago, which should be pretty obvious to most people what I've done. But nobody ever complains." — 2022-01-21, [30024797](https://news.ycombinator.com/item?id=30024797)

> "So it seems like PDF signatures are broken by design, not just implementation." — 2019-03-06, [19318434](https://news.ycombinator.com/item?id=19318434)

**6. Redaction that leaves the text in place (about 150).** Black boxes drawn over text keep the text, and each case (Manafort's lawyers, the Mueller report, Epstein releases) restarts the thread. Measured by `redaction` (redact or unredact in a PDF thread or near a PDF name): 294 hits, 41 / 41 / 212 (3.8 / 2.4 / 7.9), 6 of 12 about PDF redaction technique; the rest discuss the politics of what was redacted. The stricter `redaction_failures` (redact near black box, copy-paste, underlying text, kerning, X-ray) has 75 hits, 11 / 10 / 54, 11 of 12 on topic.

> "PDF redaction fails are everywhere and it's usually because people don't understand that covering text with a black box doesn't actually remove the underlying data." — 2025-12-24, [46370964](https://news.ycombinator.com/item?id=46370964)

> "Or they may not understand how PDF works and think that it's the same as paper." — 2025-12-23, [46370451](https://news.ycombinator.com/item?id=46370451)

> "Please don't call black overlay rectangles as "Redact" - it is maliciously misleading." — 2026-03-30, [47569622](https://news.ycombinator.com/item?id=47569622)

**7. The spec is huge and vague, and real files break it (about 140).** People cite the page count, the feature sprawl and the gap between the spec and what Acrobat accepts, which leaves every other reader guessing. Measured by `spec_complexity` (spec or format near huge, complex, bloated, insane, "756 pages"): 151 hits, 28 / 62 / 61 (2.6 / 3.6 / 2.3), 6 of 12 on topic. Plus `malformed_files` (malformed, corrupt, non-conforming, lenient parsers, broken PDFs): 150 hits, 23 / 74 / 53 (2.1 / 4.3 / 2.0), 5 of 12 on topic.

> "it's like JS + cups + PostScript + Unicode + font rendering all combined into one mega spec." — 2020-05-14, [23182813](https://news.ycombinator.com/item?id=23182813)

> "There are many ambiguities in the specification that are implemented differently between parsers, as well as implicitly accepted malformations that almost all parsers will silently accept without warning." — 2022-11-18, [33654197](https://news.ycombinator.com/item?id=33654197)

> "I stay away from things like lopdf because of how they fail to handle broken pdfs." — 2026-10-26, [50242641](https://news.ycombinator.com/item?id=50242641)

**8. AGPL traps in the free libraries (about 90).** Developers warn each other that Ghostscript, MuPDF, PyMuPDF and iText are AGPL, and that iText reads the AGPL more broadly than its text. Measured by `agpl_pdf_libs` (AGPL near Ghostscript, Artifex, MuPDF, PyMuPDF, iText, Stirling, borb or "PDF library"): 100 hits, 23 / 24 / 53 (2.1 / 1.4 / 2.0), 11 of 12 on topic.

> "I don't think many developers are aware that Ghostscript has an AGPL license" — 2019-03-12, [19364505](https://news.ycombinator.com/item?id=19364505)

> "they license their stuff under AGPL but their interpretation of AGPL is pretty extreme." — 2024-08-29, [41386759](https://news.ycombinator.com/item?id=41386759)

> "Is this PDF engine vibe-coded and derived from reading the MuPDF source code?" — 2026-10-26, [50239092](https://news.ycombinator.com/item?id=50239092)

**9. Readers disagree, and some files open only in Acrobat (about 70).** XFA forms show "Please wait" outside Acrobat, Preview breaks forms, and the same file renders differently in different viewers. Measured by `readers_disagree_acrobat_only` (XFA, "only works in Acrobat", "displays differently", Preview or pdf.js mangling a file): 99 hits, 21 / 29 / 49 (1.9 / 1.7 / 1.8), 9 of 12 on topic.

> "Fillable PDFs are evil incarnate. Apparently there are multiple ways to do it, and only Acrobat Reader is capable of making them all work." — 2019-05-03, [19820520](https://news.ycombinator.com/item?id=19820520)

> "Due to the complexity of the format, there are many PDFs that will display differently in different viewers." — 2020-08-11, [24123168](https://news.ycombinator.com/item?id=24123168)

> "The (USA) Wisconsin Dept. of Natural Resources has nearly all their regulation PDFs as these XFA non-pdfs that I cannot read." — 2026-01-23, [46726782](https://news.ycombinator.com/item?id=46726782)

**10. Adobe as a hostile vendor (about 70).** Acrobat's updaters, subscriptions, ads and, in P3, AI buttons make people distrust the format's steward. Measured by `adobe_distrust` (Adobe or Acrobat near subscription, bloat, spyware, ads, updater, AI assistant): 171 hits, 24 / 62 / 85 (2.2 / 3.6 / 3.2), 5 of 12 on topic.

> "At this point I consider the Adobe suite to be basically the same as malware." — 2020-05-14, [23182494](https://news.ycombinator.com/item?id=23182494)

> "I counted no fewer than six buttons for the AI assistant but couldn't figure out how to see chapters for the document I was reading." — 2025-10-16, [45604361](https://news.ycombinator.com/item?id=45604361)

**Smaller complaints.** Each has under about 50 on-topic comments by regex:
- *Hidden data and old revisions leak.* Regex counts are small and noisy (`metadata_hidden_history` 38 hits, 4 of 12 on topic), but the 11 forensics and leak threads (Manafort, the leaked PDF, Elsevier ×2, Snowden, Epstein ×2, X-ray, the forged PDF, PDF forgeries, "What's Hiding in Your PDF?") hold 1,414 comments. "If you need to omit edits you’d have to regenerate a fresh pdf from the root." — 2026-01-10, [46566936](https://news.ycombinator.com/item?id=46566936)
- *Tracking.* `tracking_phone_home` has 48 hits (5 of 12 on topic); the four tracking threads (Elsevier's per-download hash ×2, Elsevier's tracking viewer, Facebook crawling links in PDFs) hold 400 comments. "I would prefer that my device does not phone home to Adobe about every single PDF I decide to look at on my phone." — 2020-09-23, [24572481](https://news.ycombinator.com/item?id=24572481)
- *Fonts not embedded or not mapped to Unicode:* `fonts_embedding` 100 hits, 6 of 12 on topic.
- *Data locked in a print format:* `data_locked_in_pdf` 91 hits, 6 of 12 on topic. "From a machine-parsing perspective PDF files are a nightmare." — 2019-12-22, [21859520](https://news.ycombinator.com/item?id=21859520)
- *A paywalled standard:* `spec_access_paywall` 48 hits, 4 of 12 on topic, peaking around Adobe pulling the 1.7 spec (2022) and ISO 32000-2 becoming free (2023).

## 3. What excites or fascinates them

Hacks that push the format past its purpose draw the biggest crowds in this area, and forensic finds draw the next biggest. Points and dates are from `data/stories.jsonl`.

**Games and computers inside a PDF.** These run on the JavaScript that Chrome's PDFium and Acrobat execute, which is why the same threads fill with security worry.
- [Show HN: Tetris in a PDF](https://news.ycombinator.com/item?id=42645218), 2025-01-09, 1,289 points: the top PDF story of the decade in this data.
- [Show HN: Doom (1993) in a PDF](https://news.ycombinator.com/item?id=42678754), 2025-01-13, 369 points; [Linux Running in a PDF](https://news.ycombinator.com/item?id=42959775), 2025-02-06, 329 points (also [42891937](https://news.ycombinator.com/item?id=42891937), 183 points).
- [Breakout implemented in JavaScript in a PDF](https://news.ycombinator.com/item?id=17915296), 2018-09-05, 174 points; [Horrifying PDF Experiments](https://news.ycombinator.com/item?id=25609321), 2021-01-02, 493 points.

**Format curiosities.** [Making a PDF that's larger than Germany](https://news.ycombinator.com/item?id=39210507) (2024-01-31, 712 points), [Minimal PDF](https://news.ycombinator.com/item?id=14727662) (2017-07-08, 363 points) and [What is the smallest possible valid PDF?](https://news.ycombinator.com/item?id=22637648) (2020-03-20, 176 points). Hack stories grew from 5 stories and 543 points in P1 to 27 stories and 3,062 points in P3 (title regex over `data/stories.jsonl`).

> "projects like this are virtually never a waste of time." — 2025-01-09, [42647733](https://news.ycombinator.com/item?id=42647733)

**Forensics and redaction finds.**
- [X-ray: a Python library for finding bad redactions in PDF documents](https://news.ycombinator.com/item?id=46369923), 2025-12-23, 709 points.
- [Recreating Epstein PDFs from raw encoded attachments](https://news.ycombinator.com/item?id=46890335), 2026-02-04, 544 points; [A case study in PDF forensics: The Epstein PDFs](https://news.ycombinator.com/item?id=46886440), 2026-02-04, 409 points.
- [New information extracted from Snowden PDFs through metadata version analysis](https://news.ycombinator.com/item?id=46564762), 2026-01-10, 324 points; [Investigating a Forged PDF](https://news.ycombinator.com/item?id=45381010), 2025-09-26, 319 points.
- [Elsevier embeds a hash in the PDF metadata that is unique for each download](https://news.ycombinator.com/item?id=30082138), 2022-01-26, 343 points (reposted 2024, 284 points); [What we can deduce from a leaked PDF](https://news.ycombinator.com/item?id=31264328), 2022-05-04, 251 points; [Manafort Left an Incriminating Paper Trail Converting PDFs to Word](https://news.ycombinator.com/item?id=16447765), 2018-02-23, 334 points.

Readers liked learning that an incremental save keeps the old objects in the file:

> "So it works kind of like a soft delete — dereference instead of scrubbing the bits." — 2026-01-10, [46566455](https://news.ycombinator.com/item?id=46566455)

**Signature tricks.** [FalsiScan: make it look like a PDF has been hand signed and scanned](https://news.ycombinator.com/item?id=30024165) (2022-01-21, 939 points; first Show HN 2020-04-08, 770 points). [How to Spoof PDF Signatures](https://news.ycombinator.com/item?id=19318341) (2019-03-06, 222 points) and [Shadow attacks](https://news.ycombinator.com/item?id=23940827) (2020-07-24, 47 points) showed that incremental updates let content change under a valid signature.

**Exploits and safe-viewing tools.** [Security Flaws in Adobe Acrobat Reader Allow Gaining Root on macOS Silently](https://news.ycombinator.com/item?id=23180975) (2020-05-14, 871 points), [CVE-2024-4367 – Arbitrary JavaScript execution in PDF.js](https://news.ycombinator.com/item?id=40414718) (2024-05-20, 207 points), [Dangerzone](https://news.ycombinator.com/item?id=22516496) (2020-03-08, 317 points) and [Converting untrusted PDFs into trusted ones: The Qubes Way](https://news.ycombinator.com/item?id=42401904) (2024-12-12, 117 points).

> "It was the first time I was truly in awe of an exploit." (on FORCEDENTRY, which abused PDF's JBIG2 decoder) — 2023-07-13, [36717547](https://news.ycombinator.com/item?id=36717547)

**Inspectors, fast viewers and standards news.** [Show HN: HTML visualization of a PDF file's internal structure](https://news.ycombinator.com/item?id=43000303) (2025-02-10, 451 points), [Show HN: PDF Debugger](https://news.ycombinator.com/item?id=37710087) (2023-09-29, 157 points), [MuPDF WASM Viewer Demo](https://news.ycombinator.com/item?id=40096113) (2024-04-20, 318 points), [LaTeX is the first PDF/UA-2 compliance accessible PDF producer](https://news.ycombinator.com/item?id=40587934) (2024-06-05, 219 points), [No-cost access to ISO 32000-2](https://news.ycombinator.com/item?id=35460432) (2023-04-05, 187 points) and [ISO PDF spec is getting Brotli](https://news.ycombinator.com/item?id=46717507) (2026-01-22, 169 points).

> "The performance is astonishing. On my underpowered android, the PDF was super smooth." (MuPDF in WASM) — 2024-04-20, [40096303](https://news.ycombinator.com/item?id=40096303)

## 4. Trust, security and licensing over time

| Theme (per 1,000 PDF comments) | P1 2016–19 | P2 2020–22 | P3 2023–26 |
|---|---|---|---|
| Reader exploits (`exploits_cves`) | 12.7 | 12.5 | 10.0 |
| Sandbox and isolation (`sandbox_isolation`) | 4.1 | 5.6 | 4.4 |
| Browser viewer as the safe choice (`browser_viewer_safer`, 5 of 8 on topic) | 1.9 | 2.0 | 1.9 |
| Digital signatures (`digital_signatures`) | 3.3 | 6.5 | 3.5 |
| Image signatures (`image_signatures`) | 1.0 | 3.6 | 0.8 |
| Redaction (`redaction`) | 3.8 | 2.4 | 7.9 |
| Local processing (`privacy_local_processing`) | 5.1 | 4.8 | 8.3 |
| AGPL and PDF libraries (`agpl_pdf_libs`) | 2.1 | 1.4 | 2.0 |
| LLM or AI context (`llm_ai_context`) | 1.4 | 3.5 | 50.3 |

The universe holds 10,843 comments in P1, 17,267 in P2 and 26,882 in P3.

**Security: Adobe first, then every parser.** Worry about exploits held steady at 10–13 per 1,000 comments all decade; what moved was the culprit. In P1 and early P2 the threat was Acrobat Reader ([50 CVEs in 50 Days: Fuzzing Adobe Reader](https://news.ycombinator.com/item?id=18669720), 2018; the Acrobat root exploit, 2020), and the standard advice was to open PDFs in the browser. "Chrome/Edge PDF viewers are pretty secure." — 2022-07-06, [32002227](https://news.ycombinator.com/item?id=32002227). In P3 the browser viewers became targets too: CVE-2024-4367 in pdf.js, Ghostscript RCEs on servers (2023, 2024) and PDFium's record in Chrome.

> "So not actually all that safe since sandbox escapes happen all the time. PDF.js has had many vulnerabilities as well" — 2026-01-22, [46714752](https://news.ycombinator.com/item?id=46714752)

> "The Chrome PDF parser, originating from Foxit (now open-sourced as PDFium), has been the source of many exploits in Chrome itself" — 2026-01-22, [46713990](https://news.ycombinator.com/item?id=46713990)

The answers moved from "use another reader" to isolation: Dangerzone threads in 2020, 2024 and 2026, Qubes in 2024, and memory-safe parsers in P3. Security stories in the PDF set went from 24 stories and 92 points in P1 to 31 stories and 1,294 points in P2 and 38 stories and 1,146 points in P3.

**Trust: from faking a signature to reading what a file leaks.** P2 was the remote-signing period. Digital-signature talk doubled to 6.5 per 1,000 and image-signature talk rose from 1.0 to 3.6; 40 of the 62 image-signature hits in P2 sit in the FalsiScan and "make your PDF look scanned" threads. In P3 the focus turned to forensics: redaction talk rose from 2.4 to 7.9 per 1,000, and 137 of its 212 P3 hits came on or after 1 December 2025, the month of the Epstein releases, and the Snowden metadata study and X-ray drew 324 and 709 points. Trust stories (redaction, forensics, metadata, forgery, signatures) went from 15 stories and 624 points in P1 to 53 stories and 2,833 points in P3. Through all three periods people repeated that PDFs are not tamper-proof; in P3 they added that forgeries are rare because editing a PDF well is hard.

> "The flip side to this is that when you do come across a forged PDF (and I have!), they can be really, really effective. Nobody expects them." — 2026-09-22, [49797043](https://news.ycombinator.com/item?id=49797043)

**Privacy: local tools became the default pitch.** Talk of client-side, no-upload processing rose from about 5 per 1,000 in P1 and P2 to 8.3 in P3. Stirling-PDF mentions went from 1 in P1 to 45 in P3 (`stats/tool_mentions_by_year.json`). By 2026 commenters called client-side PDF editors the new "hello world" ([46675926](https://news.ycombinator.com/item?id=46675926), [47676029](https://news.ycombinator.com/item?id=47676029)).

**Licensing: the warnings moved from Ghostscript to iText and MuPDF.** The AGPL rate dipped in P2 and returned in P3 (2.1, 1.4, 2.0 per 1,000; 23, 24, then 53 comments). In P1, 15 of the 23 AGPL comments named Ghostscript, around Artifex's suit against Hancom. In P3, 23 of 53 named iText and 20 named MuPDF or PyMuPDF (one comment did in P1), while Ghostscript fell to 12. MuPDF mentions of any kind rose from 62 in P1 to 223 in P3 as more tools embed it. Commenters called out Show HN apps that ship MuPDF in closed products and pointed to PDFium as the permissive option.

> "For those looking for an MIT alternative, there's an embeddable solution which uses PDFium (Apache) compiled to wasm instead of MuPDF (AGPL)" — 2026-03-29, [47568333](https://news.ycombinator.com/item?id=47568333)

> "they tried to 10x our yearly license cost to the point it would have been more expensive than our AWS bill." (iText) — 2025-02-21, [43124703](https://news.ycombinator.com/item?id=43124703)

In late 2026 a new worry appeared: that an AI-written engine copies AGPL code it was trained on or shown ([50239092](https://news.ycombinator.com/item?id=50239092)).

## 5. How the decade changed

- **The PDF-versus-HTML fight cooled.** Stories framing PDF against the web fell from 946 points in P1 (Gov.uk's "HTML not PDF", 541 points) and 721 in P2 ("Still unfit for human consumption", 396) to 173 in P3. Comment rates for both sides fell after P2: "prefer HTML" from 4.3 to 1.6 per 1,000 and "prefer PDF" from 3.2 to 1.7.
- **LLMs became the context for PDF talk.** Comments naming an LLM or AI near a PDF name went from 1.4 to 50.3 per 1,000. Old complaints were restated for machines: "this just highlights the need for a more machine readable PDF alternative." — 2025-02-05, [42954660](https://news.ycombinator.com/item?id=42954660); "I wonder if some of these accessibility features help LLMs" — 2025-11-11, [45886468](https://news.ycombinator.com/item?id=45886468).
- **Security worry spread from Acrobat to every parser,** browser viewers and server-side Ghostscript included, while the overall rate stayed level.
- **Trust questions moved from forging signatures (P2) to forensics on released files (P3):** redaction failures, incremental-save history and metadata.
- **Hacks grew into the area's biggest crowd-pullers:** from 5 stories and 543 points to 27 stories and 3,062 points.
- **Accessibility and PDF/A held a steady minority** (4.7–6.4 and 2.9–4.2 per 1,000), with a shift from "PDFs are inaccessible" to "which tool can produce tagged PDF/UA or PDF/A".

## 6. What this means for a PDF platform built today

1. **Lead with safe handling of hostile files, and show it.** Reader exploits are the top complaint (about 360 on-topic comments) and safe opening the top want (about 270). A memory-safe Rust parser with limits, no JavaScript execution and repair warnings answers that directly. Because P3 commenters no longer trust browser sandboxes or PDFium by default ([46713990](https://news.ycombinator.com/item?id=46713990)), Lamina should pin and patch PDFium quickly, render in a worker, and say so in its docs.
2. **Offer a "sanitise" operation that keeps the text.** Users want active content gone without flattening to images ([46722698](https://news.ycombinator.com/item?id=46722698)); Dangerzone's raster-and-OCR approach loses text and accessibility ([22517860](https://news.ycombinator.com/item?id=22517860)). A full-rewrite save that drops JavaScript, actions, launch targets and embedded files, and reports what it removed, fills that gap.
3. **Ship a "what does this file reveal?" report.** The forensics threads (1,414 comments) and X-ray (709 points) show demand to see prior revisions, orphaned objects, metadata and text hidden under shapes. Lamina already models revisions and flags hidden text in extraction; exposing that through `laminapdf check` and the MCP server serves journalists, and a pre-publication check stops the next leak.
4. **Make redaction remove content and warn on fake redaction.** About 150 comments discuss redaction technique and about 100 ask for a tool that works. A redaction op should delete the text, glyphs and image pixels under the area, then force a full rewrite so old revisions do not survive. The editor should warn when a user draws a black rectangle over text ([38549034](https://news.ycombinator.com/item?id=38549034)).
5. **Treat signatures as evidence, with a verification report.** About 140 comments want verifiable signatures, and the spoofing and shadow-attack research found holes in incremental updates. Incremental saves that keep signatures fit this want. A report that lists the signed byte ranges and every change made after signing would tell a reader whether the file was altered, offline, as commenters ask ([45381358](https://news.ycombinator.com/item?id=45381358)).
6. **Make PDF/UA and PDF/A the easy path in generation.** Commenters say only commercial libraries produce PDF/A ([44782733](https://news.ycombinator.com/item?id=44782733)) and beg new generators for tagging ([31272485](https://news.ycombinator.com/item?id=31272485)). Lamina's permissive builder with PDF/UA-1 and PDF/A-2/3 profiles, checked by veraPDF in CI, is a direct answer. The tags also feed an HTML or Markdown twin, the "HTML version" phone and screen-reader users want (want 9).
7. **Put the licence and its provenance up front.** About 150 comments weigh licences, and AGPL traps in Ghostscript, MuPDF and iText recur in every period. MIT/Apache with no AGPL dependencies, ADR 0004's ban on MuPDF, and a stated clean-room origin for the code answer the late-2026 worry about AI-written engines copying AGPL code ([50239092](https://news.ycombinator.com/item?id=50239092)).
8. **Run locally and be tolerant of broken files.** Local processing rose to 8.3 per 1,000 in P3, but client-side PDF tools are now crowded, so correctness must set Lamina apart. Developers reject libraries that fail on broken files ([50242641](https://news.ycombinator.com/item?id=50242641)) or reject files that "legacy viewers handle fine" ([46794525](https://news.ycombinator.com/item?id=46794525)). Lamina's repair with warnings, plus a clear diagnostic for XFA-only files, meets both points.

## 7. Method and limits

**What I read.** About 1,200 comments, most to their first 300–650 characters and many in full, from about 60 threads across the decade. These included "PDF: Still unfit for human consumption", both Gov.uk HTML threads, the Acrobat root exploit, CVE-2024-4367, Dangerzone (2020, 2026), Qubes, the Adobe fuzzing and fake-job-offer threads, the Epstein, X-ray, Snowden, Manafort, Elsevier and leaked-PDF threads, the signature-spoofing, FalsiScan and forged-PDF threads, the Tetris, Doom, Linux, Breakout, Germany and Minimal PDF threads, the accessibility, PDF/A, ISO 32000-2 and Brotli threads, and the iText, MuPDF, Stirling and C# licensing threads. I also read a random sample of 105 comments from `areas/format_security.jsonl` (35 per period) and random samples by keyword (AGPL, redaction). In three rounds of spot checks I read about 1,100 more regex hits, 10–12 per theme, and in the last round I scored each as on topic or not.

**How I counted.** `tools/hn-research/counts/format-and-security/format_security_counts.py` builds its own universe from `data/threads.jsonl` and `data/comment_hits.jsonl`: 54,992 unique comments of 40 or more characters that sit in a PDF story or name PDF or a PDF tool (prep.py's rule), minus job threads. The script strips URLs first, because a link ending in `.pdf` says nothing about PDF. A theme matches when its anchor regex matches, its other regexes match within 150 characters of the anchor, and a PDF name appears within 150 characters of the anchor; some themes with distinctive words (reflow, redaction, accessibility, signatures as images, hacks) also accept a PDF-named thread title instead. Each theme's regex is in the script. `HN_DATA=<dir> python3 -I tools/hn-research/counts/format-and-security/format_security_counts.py` prints the table, `--sample-all 12 21` prints the spot-check samples, and `--extra` runs the three follow-up themes (tagged output, redaction tools, browser viewers). Story-level numbers come from title regexes over `data/stories.jsonl`, which I ran inline. Thread-level numbers count the comments in the listed story IDs. Every quote was checked by script as an exact substring of its comment, at 40 words or fewer.

**Limits.**
- Regexes miss paraphrase and catch off-topic text. Shares on topic ran from 4 of 12 (`longevity_archiving`, `metadata_hidden_history`, `spec_access_paywall`) to 11 of 12. The "about N" estimates rest on 12 samples per theme, so treat them as rough (±30%) and the ranking as an order, not a measurement.
- I estimated themes with low precision from thread-level counts instead (hidden data, tracking), and those numbers measure attention, not opinion.
- Thread-title anchoring lets in replies about politics in the Epstein and Snowden threads.
- The periods differ in length and volume: P1 covers November 2016 to 2019, P3 runs to early November 2026, and P3 has 2.5 times P1's comments. Rates per 1,000 adjust for volume but not for the many P3 comments on LLM extraction, which dilute every other theme.
- `data/stories.jsonl` was collected by a PDF search, so security or licensing stories without "PDF" in the title are missing. Reposts count once per thread.
- HN skews to developers on Mac and Linux in the US and Europe. Lawyers, government staff, publishers and blind readers appear mostly second-hand, and accessibility and archiving needs are under-represented compared with security and licensing, which developers feel directly.
