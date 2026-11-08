# PDF editors and document workflows on Hacker News, 2016–2026

*Part of [What PDF users ask for: ten years of Hacker News](../pdf-users-on-hacker-news.md). How the data was collected: [method](method.md). The work it leads to: the [trust and proof plan](../../../plans/trust-and-proof.md).*

People on HN mostly want to do small, ordinary things to PDFs: sign them, mark them up, fill in forms, make scans searchable and move pages around. They want to do these things for free, on any computer, without uploading the file. Acrobat still sets the bar for hard jobs such as redaction and odd forms, but people resent its price and weight. Over the decade the free options moved into Preview, Firefox, Edge and self-hosted tools, and since 2025 a wave of in-browser, no-upload tools has flooded Show HN. The gaps that remain are about correctness, not features: saves that break files, fake redactions, forms that only work in Acrobat, and tools sold as editors that cannot change existing text.

**How to read the counts.** Each count is the number of comments that match a theme's regex in `tools/hn-research/counts/editors/editors_counts.py`, over 56,722 PDF-related comments (2016–19: 11,180; 2020–22: 17,623; 2023–26: 27,919). Because the volume more than doubled, each period also shows the rate per 1,000 comments of that period. *Spot check* is how many of 8 random matches were on topic. Section 7 gives the details.

## 1. What users want

Ranked by how many comments discuss doing the task.

**1. Sign documents.** People want to put a signature on a PDF without printing it, and a smaller group wants real certificate signatures.
1,129 comments (2016–19: 175, 15.7 per 1k · 2020–22: 562, 31.9 · 2023–26: 392, 14.0); theme `sign_any`, spot check 7/8. Within it, 140 talk about placing an image or drawn signature (`sign_image`, 8/8) and 321 about certificate signing, eIDAS, PAdES or smart cards (`sign_crypto`, 7/8).
- "I adore the fact that I can sign PDFs so easily from the system-default viewer (and save my signature for next time)" (2022-05-09, [31318803](https://news.ycombinator.com/item?id=31318803))
- "One thing I think is really lacking from the PDF ecosystem is good open-source tools around signing." (2024-05-03, [40248518](https://news.ycombinator.com/item?id=40248518))
- "Still haven't found a way to digitally sign a pdf with a smart card on Linux, unfortunately." (2021-06-28, [27666326](https://news.ycombinator.com/item?id=27666326))

**2. Annotate and highlight.** Readers of papers, contracts and manuals want highlights, comments and pen marks that stay with the file.
780 comments (2016–19: 165, 14.8 per 1k · 2020–22: 334, 19.0 · 2023–26: 281, 10.1); `annotate`, 6/8.
- "What I would like, is to make PDF's easy to markup with highlighting, circles, comments etc." (2019-11-21, [21591006](https://news.ycombinator.com/item?id=21591006))
- "My favorite features are adding comments to highlights when reviewing a PDF" (2025-10-15, [45599814](https://news.ycombinator.com/item?id=45599814))
- "Yes, having .pdf versions of papers, that you could annotate would be very cool." (2016-12-06, [13116552](https://news.ycombinator.com/item?id=13116552))

**3. Fill in forms.** Government, tax, bank and job forms arrive as PDFs, and people want to type into them and save the result.
776 comments (2016–19: 167, 14.9 per 1k · 2020–22: 277, 15.7 · 2023–26: 332, 11.9); `form_fill`, 6/8.
- "As long as I can fill in forms and add a signature I'm sold." (2023-12-26, [38770004](https://news.ycombinator.com/item?id=38770004))
- "Finally. Its a nightmere trying to fill out a pdf form on linux." (2020-09-22, [24555702](https://news.ycombinator.com/item?id=24555702))
- "I'm struggling with many PDF forms, usually from various government agencies. They all seem to require Adobe Reader and can't open them anywhere else." (2023-08-13, [37114654](https://news.ycombinator.com/item?id=37114654))

**4. Make scans searchable.** People want an OCR text layer added to scanned PDFs, mostly with OCRmyPDF or Acrobat.
509 comments (2016–19: 81, 7.2 per 1k · 2020–22: 217, 12.3 · 2023–26: 211, 7.6); `ocr_searchable`, 6/8.
- "Scan-only PDFs are shockingly common: I download papers all the time which are scan-only." (2022-07-09, [32034892](https://news.ycombinator.com/item?id=32034892))
- "It's not super-perfect but works well enough in 99% of common cases." (on `ocrmypdf in.pdf out.pdf`, 2024-04-20, [40100713](https://news.ycombinator.com/item?id=40100713))
- "I hate to say it but Adobe Acrobat has probably one of the best scanning tools with OCR that I've seen." (2022-07-09, [32033176](https://news.ycombinator.com/item?id=32033176))

**5. Scriptable command-line tools.** Developers answer most how-to questions with pdftk, qpdf, Ghostscript or mutool, and some ask for chained steps.
433 comments name one of these tools (2016–19: 60, 5.4 per 1k · 2020–22: 162, 9.2 · 2023–26: 211, 7.6); `cli_tools`, 8/8. A further 143 talk about batch or scripted edits (`batch_script`).
- "If you find yourself needing to extract pages, split or merge pdf documents, this is wonderful." (pdftk, 2018-05-07, [17011811](https://news.ycombinator.com/item?id=17011811))
- "But pdftk is really handy if you are assembling very many pages." (2021-03-03, [26325428](https://news.ycombinator.com/item?id=26325428))
- "First merge all files then depending on output size compress to fit the size and other requirements?" (2026-01-19, [46675707](https://news.ycombinator.com/item?id=46675707))

**6. Page tools: merge, split, rotate, reorder, delete.** These are the jobs people expect free, and Preview's drag-and-drop sidebar is the reference.
356 comments (2016–19: 43, 3.8 per 1k · 2020–22: 145, 8.2 · 2023–26: 168, 6.0); `page_ops`, 7/8.
- "Just being able to cut pages and reorder them is strangely difficult for everything else." (2022-11-22, [33709653](https://news.ycombinator.com/item?id=33709653))
- "Something I still miss is a free and easy PDF tool which lets you delete, reorder and add pages from multiple PDFs." (2021-03-03, [26325701](https://news.ycombinator.com/item?id=26325701))
- "I am continually frustrated at just how difficult of a problem simple PDF operations are on Windows PCs." (2021-12-20, [29628374](https://news.ycombinator.com/item?id=29628374))

**7. Redaction that removes the text.** People want the content gone from the file, not a box drawn over it, and they cannot find free tools that do it.
328 comments mention redaction (2016–19: 54, 4.8 per 1k · 2020–22: 50, 2.8 · 2023–26: 224, 8.0); `redact`, 7/8.
- "What I miss in free alternatives is the ability to redact. Not just by placing a black bar above, but such that the text also leaves the PDF." (2025-05-03, [43881787](https://news.ycombinator.com/item?id=43881787))
- "Is there a good free tool to properly redact PDFs?" (2025-12-23, [46370144](https://news.ycombinator.com/item?id=46370144))
- "I've failed to find even a single option on linux that does real PDF redaction like adobe acrobat." (2025-11-29, [46085321](https://news.ycombinator.com/item?id=46085321))

**8. Change the existing text.** People keep receiving PDFs they need to correct, and they want to fix a word or number in place.
87 comments ask to edit existing text (`edit_text`, 5/8; 2016–19: 11 · 2020–22: 23 · 2023–26: 53, rising from 1.0 to 1.9 per 1k).
- "I'll stop trying to edit PDFs when people stop sending me PDFs that I want to edit." (2023-09-04, [37385504](https://news.ycombinator.com/item?id=37385504))
- "Why there aren't any PDF editors that support editing the actual PDF instead of adding on top of the PDF?" (2025-05-04, [43885494](https://news.ycombinator.com/item?id=43885494))

**9. Sign and mark up on a tablet or phone.** iPad and Android users want the same signing, form and pen tools they have on a Mac.
122 comments (2016–19: 24, 2.1 per 1k · 2020–22: 60, 3.4 · 2023–26: 38, 1.4); `mobile`, 6/8.
- "Filling in form PDFs, signing documents, performing some light editing is so easy in macOS and so challenging on iPadOS." (2022-05-09, [31316589](https://news.ycombinator.com/item?id=31316589))
- "I also very often need to quickly do something basic to a PDF from my phone" (2024-05-03, [40245472](https://news.ycombinator.com/item?id=40245472))
- "Pen input (Apple Pencil) is quite choppy, because browsers tend to coalesce multiple pen events together, effectively reducing the pen's polling rate." (2025-05-04, [43884012](https://news.ycombinator.com/item?id=43884012))

**10. Keep the file on my machine.** People with tax forms, IDs and contracts want tools that never upload, either in the browser or self-hosted.
85 comments praise local processing or worry about uploads (union of `privacy_local`, 6/8, and `upload_worry`, 6/8; 2016–19: 7, 0.6 per 1k · 2020–22: 10, 0.6 · 2023–26: 68, 2.4). Another 83 discuss self-hosting, mostly Stirling-PDF (`self_hosted`, 7/8; 3 → 5 → 75).
- "most of the times I fill out a form it has personal information which I don't want to risk leaking somewhere which is why I prefer local apps that don't transmit over the internet." (2017-11-21, [15745485](https://news.ycombinator.com/item?id=15745485))
- "But I prefer my wife to use this to convert/split/etc. her pdf files this way instead of using some random website or app" (Stirling-PDF at home, 2024-05-03, [40245096](https://news.ycombinator.com/item?id=40245096))
- "everything running in your browser means the PDF never leaves your device, so it's 100% private." (2025-05-03, [43882265](https://news.ycombinator.com/item?id=43882265))

**11. Shrink files to fit an upload limit.** People need a PDF under a portal's or email's size cap, and most tools disappoint.
62 comments (2016–19: 8 · 2020–22: 15 · 2023–26: 39, 0.7 → 1.4 per 1k); `compress`, 6/8.
- "Is there a .pdf tool which allows compression to a defined file size?" (2021-03-03, [26325269](https://news.ycombinator.com/item?id=26325269))
- "every “shrink” tool we tried barely compressed the size, and some even made it larger!" (2025-05-04, [43884344](https://news.ycombinator.com/item?id=43884344))

## 2. What they complain about most

**1. Acrobat costs too much and gets in the way.** The complaints mix the subscription, nags to sign in, upsells, slowness and size.
258 comments (union of `acrobat_price`, 106, 6/8, and `acrobat_bloat`, 156, 8/8; 2016–19: 40, 3.6 per 1k · 2020–22: 108, 6.1 · 2023–26: 110, 3.9).
- "Acrobat Reader is one of the more poorly engineered programs I’ve used. And it recently asked me to sign in an account and give Adobe money to open a PDF??" (2025-10-15, [45599763](https://news.ycombinator.com/item?id=45599763))
- "The free version of Acrobat is bloated, full of spam, and can’t actually do all of this (eg splitting and merging)" (2023-08-13, [37111115](https://news.ycombinator.com/item?id=37111115))
- "Most people mainly use only PDf merge & split & add comments & signing - and pay for this a monthly Adobe subscription of 29,99 USD last time I checked." (2026-01-19, [46676594](https://news.ycombinator.com/item?id=46676594))

**2. No full editor on Linux, and no open-source Acrobat.** Linux users stitch together Okular, Xournal++, LibreOffice Draw and command-line tools, and still miss forms, signing and redaction.
139 comments (union of `linux`, 5/8, and `want_oss_acrobat`; 2016–19: 27, 2.4 per 1k · 2020–22: 49, 2.8 · 2023–26: 63, 2.3).
- "All the Linux PDF viewers suck -- they can't even save a fill-in PDF form or insert a signature." (2020-05-31, [23367570](https://news.ycombinator.com/item?id=23367570))
- "Adobe Acrobat. Nothing in the open-source world has the same share of easiness in editing PDF files. (Sorry, Okular.)" (answering which project has no open-source alternative, 2019-12-28, [21897359](https://news.ycombinator.com/item?id=21897359))
- "I am always surprised there is absolutely nothing like the Adobe Acrobat Pro on the open source space." (2024-05-13, [40340490](https://news.ycombinator.com/item?id=40340490))

**3. Recipients demand a printed, hand-signed scan.** Banks, car dealers and agencies reject pasted signatures, so people fake the scan.
124 comments (`print_sign_scan`, 8/8; 2016–19: 3, 0.3 per 1k · 2020–22: 85, 4.8 · 2023–26: 36, 1.3).
- "the companies in question complain that it can't possibly be a handwritten signature because the document didn't look printed and scanned (???)" (2022-01-21, [30026403](https://news.ycombinator.com/item?id=30026403))
- "My bank sometimes requires a “wet” signature and have rejected digitally signed PDFs from MacOS Preview before." (2020-04-08, [22812825](https://news.ycombinator.com/item?id=22812825))
- "Especially if you can “select” all the text, and when you select the signature you see a nice box around it. It’s then too obvious it was added as an image." (2022-01-21, [30024762](https://news.ycombinator.com/item?id=30024762))

**4. Forms that only work in Acrobat.** XFA forms show only an Adobe placeholder message in other viewers, and many forms are not fillable at all.
121 comments (union of `xfa`, 8/8, and `forms_dont_work`, 8/8; 2016–19: 23, 2.1 per 1k · 2020–22: 46, 2.6 · 2023–26: 52, 1.9).
- "PDF is no longer a single format. You have to specify what type of PDF. Many will not open in anything but Acrobat." (2025-10-15, [45599229](https://news.ycombinator.com/item?id=45599229))
- "USCIS forms still use XFA PDFs, which don’t let you edit in most browsers. Even with Adobe, fields break, and getting the signature is hard." (2026-03-09, [47306129](https://news.ycombinator.com/item?id=47306129))
- "Adobe Reader is pretty mandatory if you are doing anything more with a PDF than hitting the print key." (2017-01-13, [13395312](https://news.ycombinator.com/item?id=13395312))

**5. Annotations that do not travel.** Notes made in one app show up wrong in another, get lost in sync, or cannot be exported.
119 comments (`annot_interop`, 5/8; 2016–19: 25, 2.2 per 1k · 2020–22: 45, 2.6 · 2023–26: 49, 1.8).
- "Preview is genuinely very good, but it doesn’t handle annotations made in Acrobat very well." (2025-10-16, [45602082](https://news.ycombinator.com/item?id=45602082))
- "Does anybody know a Linux PDF viewer which can add annotations? That is, annotations that properly show up in other PDF viewers like Adobe's." (2017-04-13, [14108346](https://news.ycombinator.com/item?id=14108346))
- "Is it possible to export highlights and notes as Markdown? That would let me dump my notes into Obsidian." (2022-07-04, [31972968](https://news.ycombinator.com/item?id=31972968))

**6. Editing text is hard, and most tools sold as editors only add layers.** Tools sold as PDF editors mostly add text boxes on top; changing existing words fails on subset fonts and layout.
98 comments (`edit_hard`, 5/8; 2016–19: 37, 3.3 per 1k · 2020–22: 21, 1.2 · 2023–26: 40, 1.4).
- "I guess adding stuff is much easier than editing existing stuff which you didn't add" (2025-05-03, [43881618](https://news.ycombinator.com/item?id=43881618))
- "PDFs commonly only include the glyphs from the font that are actually used in the document." (2025-05-03, [43883065](https://news.ycombinator.com/item?id=43883065))
- "\"Edit PDFs\" is a bit misleading because it's only annotations. You cannot, for example, remove or edit existing text." (on Firefox, 2024-05-18, [40398095](https://news.ycombinator.com/item?id=40398095))

**7. Redactions that leave the text in the file.** Black boxes over live text keep leaking, from the Manafort filing to the Epstein files, and incremental saves keep earlier revisions.
79 comments (`redact_fail`, 8/8; 2016–19: 14, 1.3 per 1k · 2020–22: 14, 0.8 · 2023–26: 51, 1.8).
- "Placing a black box on the text isn’t a redaction any more than placing a sticky note would be." (2025-12-24, [46374309](https://news.ycombinator.com/item?id=46374309))
- "Ahh, another example of PDF retraction that puts an opaque black box over something, yielding to the highly sophisticated \"Highlight the text\" attack." (2018-12-09, [18642369](https://news.ycombinator.com/item?id=18642369))
- "meaning the previous revision of the file is available inside of the PDF itself." (on a `/Prev` pointer in an Epstein file, 2025-12-23, [46370872](https://news.ycombinator.com/item?id=46370872))

**8. Saving breaks the file.** Preview in particular has scrambled OCR text, broken form styling and lost annotations on save.
36 comments (`save_breaks`, 5/8; 2016–19: 2 · 2020–22: 24 · 2023–26: 10). The 2020 story "Preview in macOS Big Sur is destroying PDFs" drew 359 points and 317 comments ([25447830](https://news.ycombinator.com/item?id=25447830)).
- "After applying a signature to a PDF in Preview, the searchable text layer becomes scrambled so that numbers are no longer searchable or copy-able." (2023-12-26, [38775028](https://news.ycombinator.com/item?id=38775028))
- "saving a PDF with forms in Preview would set some style setting so that in most other readers, the form fields would have no background color and use white text, making them unreadable." (2020-12-16, [25448297](https://news.ycombinator.com/item?id=25448297))
- "After about 70 pages or so preview crashed and the file became corrupted with no way to recover the annotation or comments." (2025-10-16, [45603078](https://news.ycombinator.com/item?id=45603078))

**9. Online tools want your private files.** Free web tools upload the document, hit size limits or paywalls, and often only annotate.
36 comments (`upload_worry`, 6/8; 2016–19: 3 · 2020–22: 6 · 2023–26: 27).
- "other tools can edit existing text, but they upload your PDF to their servers, so it's not private if that's something you care about." (2025-05-03, [43882306](https://news.ycombinator.com/item?id=43882306))
- "I don't quite see how a web-based PDF converter has a \"privacy focus\". Am I not sending my PDF to the server for processing?" (2025-01-03, [42585412](https://news.ycombinator.com/item?id=42585412))
- "Aside from the privacy concerns of uploading tax forms or personal documents, they usually hit you with a \"File too large\" error or a paywall after two uses." (2026-03-09, [47313393](https://news.ycombinator.com/item?id=47313393))

## 3. What excites them

Five kinds of release drew large threads. Points and comment counts come from `data/stories.jsonl`.

**Free editing inside the browser.** "Embedded PDF viewer in Firefox 81 supports filling forms" (2020-09-22, 1,093 points, 375 comments, [24554581](https://news.ycombinator.com/item?id=24554581)) is the largest editor story of the decade. Later Firefox steps drew smaller threads: "Edit PDFs for Free with Firefox PDF Editor" (2024-05-16, 112 points, [40383879](https://news.ycombinator.com/item?id=40383879)). Commenters treat each step as a reason to drop Acrobat or Chrome: "FINALLY! Now I can finally uninstall Chrome." (2020-09-22, [24554664](https://news.ycombinator.com/item?id=24554664)) and "That's a fantastic feature. No more Adobe Acrobat for me!" (Firefox 106, 2022-10-18, [33247361](https://news.ycombinator.com/item?id=33247361)).

**Tools that make a PDF look hand-signed and scanned.** Five stories on faking a scan drew 3,050 points between them: FalsiScan (2020-04-08, 770 points, [22811653](https://news.ycombinator.com/item?id=22811653); 2022-01-21, 939 points, 390 comments, [30024165](https://news.ycombinator.com/item?id=30024165)), "Make Your PDF Look Scanned" (2020-05-12, 607, [23157408](https://news.ycombinator.com/item?id=23157408); 2022-04-19, 579, [31083432](https://news.ycombinator.com/item?id=31083432)) and a WASM version (2026-06-20, 155, [48611513](https://news.ycombinator.com/item?id=48611513)). The demand is a protest against print-sign-scan rules, not against digital signing.

**Local, no-upload tool suites.** "PDF Tool – Modify PDFs in the browser without uploading" (2023-08-13, 522 points, [37110628](https://news.ycombinator.com/item?id=37110628)), Stirling-PDF (2023-12-25, 507 points, 231 comments, [38765627](https://news.ycombinator.com/item?id=38765627); 2024-05-02, 366, [40242639](https://news.ycombinator.com/item?id=40242639)), "Show HN: Free, in-browser PDF editor" (2025-05-03, 791 points, 182 comments, [43880962](https://news.ycombinator.com/item?id=43880962)), "Local PDF Tools – Powered by WebAssembly" (2021-03-03, 227, [26323966](https://news.ycombinator.com/item?id=26323966)) and Pdfwithlove (2026-01-19, 177 points, 131 comments, [46675231](https://news.ycombinator.com/item?id=46675231)). Praise for Stirling ran high: "This looks like a wonderful tool that solves a lot of problems that existing tools don't typically put together." (2023-12-26, [38770523](https://news.ycombinator.com/item?id=38770523)). By 2026 the excitement had turned to fatigue: "Seems like Clientside PDF editors are the new \"hello world\" app these days." (2026-01-19, [46675926](https://news.ycombinator.com/item?id=46675926)) and "All with similar design, similar implementation, similar HN post. Literally AI slop." (2026-01-19, [46676300](https://news.ycombinator.com/item?id=46676300)).

**Embeddable annotation and redaction for apps.** "Show HN: I built a free alternative to Adobe Acrobat PDF viewer" (2025-08-14, 357 points, [44901683](https://news.ycombinator.com/item?id=44901683)) and "Pdf.js Express – PDF annotation, e-signatures, and form filling" (2020-04-02, 334 points, [22763656](https://news.ycombinator.com/item?id=22763656)) drew developers who need markup inside their own products: "Once you need real PDF markup tools, your options are either building them yourself or using a subscription-based PDF editor - neither comes cheap." (2025-08-14, [44906812](https://news.ycombinator.com/item?id=44906812)).

**Checks, signing and OCR from the command line, then AI.** "X-ray: a Python library for finding bad redactions in PDF documents" (2025-12-23, 709 points, [46369923](https://news.ycombinator.com/item?id=46369923)), "Digitally sign PDF files from your commandline – open-pdf-sign" (2022-12-19, 252, [34051652](https://news.ycombinator.com/item?id=34051652)), "OCRmyPDF: Add an OCR text layer to scanned PDF file" (2022-07-08, 240, [32028752](https://news.ycombinator.com/item?id=32028752)), "So you want to modify the text of a PDF by hand" (with qpdf, 2023-09-03, 325, [37368148](https://news.ycombinator.com/item?id=37368148)) and "Spotlight on pdfly" (2025-10-13, 338, [45566139](https://news.ycombinator.com/item?id=45566139)). AI editing arrived late and small: "Nano PDF – A CLI Tool to Edit PDFs with Gemini's Nano Banana" (2025-11-29, 176 points, [46090619](https://news.ycombinator.com/item?id=46090619); "This is nuts and I absolutely love this.", [46090751](https://news.ycombinator.com/item?id=46090751)) and "Filling PDF forms with AI using client-side tool calling" (2026-05-02, 60 points, [47984675](https://news.ycombinator.com/item?id=47984675)).

## 4. Tools and sentiment over time

Mentions are comments that name the tool and a PDF word; the rate is per 1,000 comments of the period. *Hand-coded* is my reading of a seeded random sample of mentions (P praise or a capability the writer values, N complaint, the rest neutral); the IDs are in the script's `HAND` table. *Lexicon net* is (positive − negative) / (positive + negative) over a crude word list within 200 characters of the name; it is a weak signal.

| Tool | 2016–19 | 2020–22 | 2023–26 | Evidence of direction |
|---|---|---|---|---|
| Acrobat / Reader | 345 (30.9) | 733 (41.6) | 834 (29.9) | hand-coded P/N 2/4, 5/8, 4/7 (n = 20 each) |
| Preview (macOS) | 95 (8.5) | 275 (15.6) | 200 (7.2) | hand-coded P/N 8/3, 10/1, 9/2 (n = 15 each) |
| Browser built-ins editing | 12 | 20 | 29 | hand-coded P/N 7/1, 5/1 (n = 12, last two periods) |
| Okular | 103 (9.2) | 193 (11.0) | 130 (4.7) | lexicon net +0.24, +0.59, +0.52 |
| Xournal(++) | 2 | 46 (2.6) | 32 (1.1) | lexicon net +0.89, +0.64 |
| LibreOffice Draw (as PDF editor) | 15 | 31 | 38 | lexicon net +0.56, +0.78 (last two) |
| Stirling-PDF | 1 | 2 | 45 (1.6) | hand-coded P/N 7/4 (n = 15) |
| pdftk / qpdf / Ghostscript | 34 / 18 / 99 | 51 / 25 / 190 | 60 / 59 / 228 | qpdf lexicon net +1.00 in the last two periods |
| Foxit / PDF-XChange / Master PDF Editor | 59 / 7 / 3 | 89 / 17 / 8 | 78 / 23 / 11 | PDF-XChange net +0.14, +0.71, +0.82 |
| Smallpdf / iLovePDF / PDF24 / Sejda | 2 | 6 | 36 (1.3) | lexicon net +1.00, small n |
| DocuSign | 12 | 40 (2.3) | 35 (1.3) | hand-coded P/N 3/3, then 0/6 (n = 12) |

**Acrobat: resented, still the fallback.** Complaints outnumber praise in every hand-coded sample, and Acrobat's mention rate peaked in 2020–22, when cost-or-bloat complaints also peaked (6.1 per 1k). Even so, commenters keep naming it as the only tool for redaction, odd forms and OCR. One firm paid for licences for that reason: "This was basically the only reason we were willing to cough up like $400 for each Acrobat license for a few hundred people." (2025-12-24, [46379121](https://news.ycombinator.com/item?id=46379121)). Another verdict sums up the period: "I hate Acrobat but it is still the best pdf program on Windows." (2025-10-18, [45631155](https://news.ycombinator.com/item?id=45631155)). Some defend its robustness: "Acrobat may be bloated but there also seems to be some robust code there covering edge cases other readers mess up." (2025-08-25, [45015367](https://news.ycombinator.com/item?id=45015367)).

**Preview: the most loved tool, with a trust problem.** Praise dominates every hand-coded sample, and people cite it as the model for page tools and signing: "Preview is both crazy good and free." (2022-11-22, [33709622](https://news.ycombinator.com/item?id=33709622)). Its saves, however, are the main source of broken-file stories (the Big Sur thread in 2020, signatures scrambling text in 2023, crashes losing annotations in 2025), and forms filled in it look wrong elsewhere: "I can always tell with some forms that they've been filled out in Preview on Mac due to how they mangle it." (2025-08-25, [45014721](https://news.ycombinator.com/item?id=45014721)).

**Browsers: from viewer to good-enough editor.** Firefox gained form filling in 2020 and annotation, text and signatures from 2022, and Edge became a favourite for pen markup. Hand-coded mentions are mostly positive ("ms-edge is surprisingly good for quick pdf annotations and my preferred PDF reader on Linux.", 2023-08-13, [37112473](https://news.ycombinator.com/item?id=37112473)), but people notice the limits: "Firefox still only handles basic PDFs, anything beyond that, like fillable forms, is going to require a 3rd party solution." (2021-03-19, [26517321](https://news.ycombinator.com/item?id=26517321)).

**Linux tools: liked, but piecemeal.** Okular and Xournal++ draw warm comments ("Xournal++ is amazing for annotating PDFs.", 2023-12-27, [38778688](https://news.ycombinator.com/item?id=38778688)). Each one covers only part of the job ("There is no support for redacting text properly on Okular.", 2023-12-26, [38770825](https://news.ycombinator.com/item?id=38770825)). LibreOffice Draw is praised for real editing and blamed for damage ("LibreOffice Draw kinda try to do it but also corrupts the file IME.", 2023-12-26, [38775737](https://news.ycombinator.com/item?id=38775737)). The paid Master PDF Editor and PDF-XChange (on Windows) collect steady praise from people who gave up on free tools.

**Stirling-PDF: new in 2023, popular, questioned.** It went from 3 mentions before 2023 to 45 after. Praise centres on keeping files at home. Criticism centres on running Docker for a simple job ("Why would I run a docker container, a webserver, start a browser, navigate webpages... just to do some operations on a pdf locally?", 2023-12-26, [38769262](https://news.ycombinator.com/item?id=38769262)) and on its machine-written early code.

**Command-line tools: quiet, steady trust.** pdftk, qpdf and Ghostscript appear in answers more than in complaints, and qpdf mentions tripled (18 → 59). Their weak spot is ordinary users; one commenter self-hosts a web front end so that a family member can avoid the terminal ([40245096](https://news.ycombinator.com/item?id=40245096)).

**DocuSign: tolerated, then mocked.** In 2020–22 people credited its legal checklist and polish ("TBH Docusign also has more polished UI", 2021-11-28, [29365518](https://news.ycombinator.com/item?id=29365518)). By 2023–26 every coded mention was negative: "It's annoying to pay $11 a MONTH to freaking sign a PDF" (2026-10-12, [50056992](https://news.ycombinator.com/item?id=50056992)) and "not \"I pinky swear I'm John Smith\" DocuSign." (2025-01-13, [42682218](https://news.ycombinator.com/item?id=42682218)).

## 5. How the decade changed

- **Signing surged with the pandemic, then fell back.** Signing comments doubled from 15.7 to 31.9 per 1k in 2020–22 and dropped to 14.0 in 2023–26; print-sign-scan complaints went from 0.3 to 4.8 to 1.3, and certificate signing from 3.9 to 9.1 to 4.2.
- **Redaction became the headline failure.** Redaction comments rose from 2.8 to 8.0 per 1k between the last two periods, driven by the Epstein files in December 2025 (the X-ray story drew 709 points). The talk shifted from how to draw a box to the fact that the hidden text and earlier revisions stay in the file.
- **No-upload processing went from a niche to the default pitch.** Local-processing and upload-worry comments rose from 0.6 to 2.4 per 1k. Show HN posts for PDF tools held at 3–7% of PDF stories through 2024, then reached 10.1% in 2025 and 13.7% in 2026 (to 6 November); posts whose titles mention local, in-browser, WASM, private or no-upload processing went from 0–3 a year to 16 in 2025 and 53 in 2026. Their median score in 2023–26 was 2 points.
- **Free editing moved into the OS and the browser, and self-hosting arrived.** Preview, Firefox and Edge now cover forms, signatures and page tools for most people. Stirling-PDF brought a self-hosted suite (self-hosting comments 0.3 → 2.7 per 1k). Acrobat's mention rate fell from 41.6 to 29.9 per 1k after its 2020–22 peak.
- **AI arrived in editing only in the last two years.** AI-for-editing comments went from 0 and 0 to 17, alongside Nano PDF and client-side AI form filling.

## 6. What this means for a PDF editor built today

1. **Compete on correctness, because the feature grid is a commodity.** In 2026 alone, 110 Show HN posts offered PDF tools, and commenters call them hello-world apps. The failures they report are specific: a merge that drops links ("I noticed that this tool doesn't preserve links within PDFs.", 2025-05-04, [43888208](https://news.ycombinator.com/item?id=43888208)), compression that rasterizes every page ([43895227](https://news.ycombinator.com/item?id=43895227)), and tools sold as editors that only annotate ([40398095](https://news.ycombinator.com/item?id=40398095)). Lamina PDF should keep links, outlines, form fields and text through every page operation, and it should compress by re-encoding images and fonts, never by turning text into pictures.

2. **Make every save safe and boring.** The saving complaints (36, plus the 317-comment Big Sur thread) all describe a tool that rewrote parts of the file it did not need to touch. Incremental saves that leave the original bytes intact, as ADR 0007's save planner does, answer this directly. The test suite should open each saved file in Acrobat, Preview, pdf.js and PDFium and check that text layers, form appearances and annotations survive ([25448297](https://news.ycombinator.com/item?id=25448297), [45602082](https://news.ycombinator.com/item?id=45602082)).

3. **Ship redaction as a real operation, plus a check for fake redaction.** Redaction must delete the covered text, images and annotation content and then force a full rewrite that drops earlier revisions, because `/Prev` pointers and incremental saves leaked history ([46370872](https://news.ycombinator.com/item?id=46370872), [46371090](https://news.ycombinator.com/item?id=46371090)). A `laminapdf check` rule that flags boxes over live text and kept revisions meets the demand behind X-ray's 709 points. Linux users say no tool on their platform does real redaction ([46085321](https://news.ycombinator.com/item?id=46085321)). The licence matters here too: one in-browser editor left redaction out because its library was AGPL ("I didn't include it in this tool for now due to the APGL license of the library that enables the redaction", 2025-05-03, [43882632](https://news.ycombinator.com/item?id=43882632)), so an MIT/Apache core is an advantage.

4. **Offer both kinds of signature, under different names.** Most demand is for placing an ink signature (140 comments, and 3,050 points of fake-scan stories). A smaller, steady group wants certificate signing with PAdES, eIDAS identities and smart cards (321 comments) and finds no open-source tool for it ([40248518](https://news.ycombinator.com/item?id=40248518)). The interface should name them differently, because commenters confuse them constantly ([22812571](https://news.ycombinator.com/item?id=22812571)). Validation should show which revision a signature covers, since "PDF signatures are broken by design" was the reaction to the 2019 spoofing research ([19318434](https://news.ycombinator.com/item?id=19318434)).

5. **Treat forms as a first-class job, including XFA.** Fill-and-save must produce forms that look right in other viewers (121 comments say forms break outside Acrobat). For XFA, the reader should say plainly that the file is an XFA form instead of showing Adobe's placeholder page, and convert static XFA to AcroForm where it can; one commenter did that in seconds for a Canadian tax form ([24575185](https://news.ycombinator.com/item?id=24575185)).

6. **Make the command line and MCP server the place for chained jobs.** Developers already script pdftk, qpdf and Ghostscript (433 comments), and both developers and other users ask for chains such as merging files and then compressing them to a size limit ([46675707](https://news.ycombinator.com/item?id=46675707), [26325269](https://news.ycombinator.com/item?id=26325269)). Page ranges, a target-size compression option and folder batches in the `laminapdf` CLI cover this. AI form filling is starting to appear ([47984675](https://news.ycombinator.com/item?id=47984675)); MCP tools that fill and save a form locally fit that need without an upload.

7. **Use the web reader to close the Linux and tablet gaps.** Linux users (139 comments) and tablet and phone users (122) lack a single tool for forms, signing and markup. A browser editor that runs locally reaches both, but only if pen input is smooth; browsers coalesce pointer events unless the app reads them all ([43884012](https://news.ycombinator.com/item?id=43884012)).

8. **Be honest about text editing.** Users want to fix words in place (87 comments ask, 98 complain), but subset fonts often lack the needed glyphs ([43883065](https://news.ycombinator.com/item?id=43883065)). A small fix-a-word feature that checks the font's glyphs and warns when it must substitute a font would beat tools that promise Word-like editing and fail.

## 7. Method and limits

**What I read.** I read about 1,400 comments, most as 250–450-character excerpts, across about 60 editor threads (the largest first, from every year) and seeded random samples of `areas/editors.jsonl` for each year from 2016 to 2026. I also read the full text of about 120 candidate quotes (82 are used here, from 82 comments), about 650 regex-match snippets for spot checks and 168 snippets for hand-coded sentiment. Every quote above was checked by script to appear verbatim in `data/threads.jsonl` or `data/comment_hits.jsonl` and to run 40 words or fewer.

**How I counted.** `tools/hn-research/counts/editors/editors_counts.py` (run as `HN_DATA=<dir> python3 -I -B tools/hn-research/counts/editors/editors_counts.py`) builds the base the same way `tools/hn-research/prep.py` does: every unique comment of at least 40 characters that sits in a PDF story or mentions a PDF term. That gives 56,722 comments, which contain all 9,069 comments of `areas/editors.jsonl`. I counted over the full base, with untruncated text, because the area file's regex missed many OCR and Acrobat-pricing comments. Each theme is one regex, usually with a second regex the comment must also match and a rule that the comment itself names a PDF or PDF tool; the script prints how many matches also fall in `areas/editors.jsonl`. A comment counts once per theme, and unions count it once across their members. `--samples THEME N SEED` prints random matches; the spot-check scores above come from 8 matches per theme with the final regexes:

| Spot check (on topic of 8) | Themes |
|---|---|
| 8/8 | `cli_tools`, `acrobat_bloat`, `print_sign_scan`, `sign_image`, `xfa`, `forms_dont_work`, `redact_fail` |
| 7/8 | `sign_any`, `page_ops`, `redact`, `self_hosted`, `sign_crypto` |
| 6/8 | `annotate`, `form_fill`, `ocr_searchable`, `mobile`, `privacy_local`, `upload_worry`, `compress`, `acrobat_price` |
| 5/8 | `edit_text`, `edit_hard`, `linux`, `annot_interop`, `save_breaks` |

**Limits.**
- The counts measure comments, not people, and the regexes still let some off-topic text through (precision 5/8 to 8/8). Treat counts as rough sizes and focus on the ranking and the per-period rates.
- Eight-match spot checks give only rough precision.
- The lexicon sentiment ignores negation and sarcasm. The hand-coded samples are small (12–20 per tool and period) and coded by one reader.
- HN skews towards developers, Mac users and privacy-minded people. That likely inflates Preview, Linux and command-line talk and deflates Windows office users, lawyers and construction users (Bluebeam appears only 14 times).
- Single events drive spikes: the pandemic (signing, 2020–22) and the Epstein files (redaction, December 2025).
- 2026 covers only January to 6 November.
- The story set comes from keyword searches, so related stories without `PDF` in the title can be missing (the DocuSeal and "Why is Docusign a $50B company?" stories are not in `data/stories.jsonl`, so they have no points here).
- The supplied `stats/tool_mentions_by_year.json` counts `acrobat` without word boundaries: 166 of its 523 Acrobat comments in 2016–19 are about acrobatics. My tool table uses `\bacrobat\b`.
