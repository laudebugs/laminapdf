# PDF readers on Hacker News, Nov 2016 – Nov 2026: users want fast, safe, quiet readers, and Acrobat moved the other way

*Part of [What PDF users ask for: ten years of Hacker News](../pdf-users-on-hacker-news.md). How the data was collected: [method](method.md). The work it leads to: the [trust and proof plan](../../../plans/trust-and-proof.md).*

Over ten years, Hacker News commenters asked PDF readers for the same few things. They want a reader that opens at once, reads well on tablets and phones, takes annotations, and fills forms without Acrobat. They complain most about security holes and slow, bloated software, and more and more about Acrobat's upsells and, since 2023, its AI prompts. The biggest shift is Acrobat itself. In a hand-labelled sample, its positive comments fell from 5 in 20 to 1 in 20 while its negative ones rose from 6 to 9. Over the same years, browser viewers gained forms, annotations and editing, and SumatraPDF stayed the most-loved reader of the decade.

All counts below come from the scripts in `tools/hn-research/counts/readers/` (see section 7). They are counted over the 7,277 comments in `areas/readers.jsonl`: 1,581 in 2016–19, 2,959 in 2020–22 and 2,737 in 2023–26. Percentages are shares of each period's comments. "Spot-check" means I read a random sample of the matches and give the share that were on topic.

## 1. What users want

**1. A good tablet or e-ink device for PDFs.** People want a large, light screen for long PDFs, papers and textbooks, and they judge each device by how it handles two-column pages and pen notes.
Count: 1,020 comments, 14.2% → 16.0% → 11.8%. Theme `tablet_eink`: device names (iPad, reMarkable, Boox, Kindle, Kobo, e-ink, KOReader…) within 150 characters of "PDF", "paper", "textbook" or "annotate". Spot-check: about 8 in 10 on topic.
- "I have about 100 research papers as PDFs on my remarkable 2. It's delightful to read and annotate on it!" ([27683855](https://news.ycombinator.com/item?id=27683855), 2021-06-30)
- "Real world PDFs are mostly unreadable on eInk kindles. The screen size doesn’t work for most 2 column PDFs. It is fairly painful." ([22173233](https://news.ycombinator.com/item?id=22173233), 2020-01-28)
- "iPad Pro 12.9" is an excellent pdf viewer. Especially in portrait mode." ([22211180](https://news.ycombinator.com/item?id=22211180), 2020-02-01)

**2. Annotation and highlighting.** Readers want to highlight and write on PDFs, often with a pen, and they want other viewers to show those notes.
Count: 679 comments, 10.1% → 10.6% → 7.5%. Theme `annotation`: annotate, highlight, markup, sticky note, pencil, stylus near a reader name or "PDF". Spot-check: about 8 in 10.
- "My use case is making lots of highlights in textbook PDFs and I usually can't highlight for long before it beachballs." ([23185666](https://news.ycombinator.com/item?id=23185666), 2020-05-14)
- "Last time I checked Zathura did not support annotations. This is too bad. It would be a nice program otherwise for reading. I need annotations though." ([23537307](https://news.ycombinator.com/item?id=23537307), 2020-06-16)
- "subject-matter experts sent their comments as PDF annotations. Most FOSS viewers can't show them." ([48981047](https://news.ycombinator.com/item?id=48981047), 2026-07-20)

**3. Reflow for phones and e-readers.** Readers want a mode that turns fixed pages into one readable column. Most of these comments are complaints that fixed pages fail on small screens.
Count: 501 comments, 7.7% → 7.3% → 6.0%. Theme `reflow_small_screen`: reflow, reading mode, Liquid Mode, k2pdfopt, small or phone screen, two-column, crop margins, pinch-to-zoom near "PDF" or a reader name. Spot-check: about 8 in 10.
- "The agony of PDFs on mobile phone screens is just awful. Please, world, let there be portable documents that can reflow text so as to render at a size that doesn't make my eyes bleed." ([22174002](https://news.ycombinator.com/item?id=22174002), 2020-01-28)
- "I’d love it if any PDF reader could simply reformat PDFs to be single column so they would be viewable on mobile devices." ([40314791](https://news.ycombinator.com/item?id=40314791), 2024-05-10)
- "The "reader mode" re-flows the PDF into a screen-width single column like an e-book." ([37008014](https://news.ycombinator.com/item?id=37008014), 2023-08-05; about Xodo)

**4. Speed and a small install.** People praise readers that open instantly and install small, and they name SumatraPDF, MuPDF and Zathura most.
Count: 253 comments praise speed (3.9% → 3.2% → 3.5%), theme `speed_light`: fast, snappy, lightweight, opens instantly, small footprint within 120 characters of a reader name. Spot-check: about 7 in 10. With the 260 comments that complain of slowness (complaint 2), 470 comments are about speed.
- "its performance blows both Adobe Reader (not an achievement) and Foxit (something of an achievement) out of the water!" ([37995366](https://news.ycombinator.com/item?id=37995366), 2023-10-24; about SumatraPDF)
- "I still install Sumatra PDF on every Windows PC I have to work on. It's a habit I've kept from the days when Acrobat Reader took like 30 seconds to start." ([27971453](https://news.ycombinator.com/item?id=27971453), 2021-07-27)
- "Something refreshing about having PDFs open instantly." ([35067413](https://news.ycombinator.com/item?id=35067413), 2023-03-08)

**5. Forms and signatures without Acrobat.** People want to fill a form and add a signature in the reader they already use. These two tasks are the main reason they keep Acrobat installed.
Count: 283 comments, 2.8% → 5.0% → 3.4%: the union of `form_fill` (fill forms, fillable, XFA, AcroForm; spot-check 9 in 10) and `sign` (sign a PDF, signatures, e-sign near a reader name; 8 in 10). The 2020–22 peak includes the Firefox 81 forms thread.
- "I still need Adobe to sign and fill forms. Wish Sumatra can do those too :(" ([37994549](https://news.ycombinator.com/item?id=37994549), 2023-10-24)
- "it’s maybe surprising but it is a VERY REAL pain in the Windows world to find a pdf reader that also allows you to edit forms, that doesn’t also come with malware or adware" ([24558073](https://news.ycombinator.com/item?id=24558073), 2020-09-22)
- "I can fill out forms, draw signatures and print to paper or PDF without installing new software or go looking for a web service." ([15746310](https://news.ycombinator.com/item?id=15746310), 2017-11-21; about Preview)

**6. Free software with no strings.** People want a free viewer without ads, tracking or a push to pay.
Count: 171 comments, 2.8% → 2.3% → 2.1%. Theme `open_source_free`: open source, FOSS, free software, libre near a reader name. Spot-check: about 6 in 10; the rest are lists of apps.
- "there is a dearth of free, high-quality PDF viewers that aren't bloated or filled with ads or spyware or trying to get you to upgrade to a paid version of their software." ([24562087](https://news.ycombinator.com/item?id=24562087), 2020-09-23)
- "Which PDF readers contain tracking? Anyway, there are several open-source ones that don't." ([24555888](https://news.ycombinator.com/item?id=24555888), 2020-09-22)

**7. One viewer for many formats.** People like a viewer that also opens EPUB, DjVu, XPS and comic archives. This is a large part of SumatraPDF's and Okular's appeal.
Count: 164 comments, 3.0% → 2.1% → 2.0%. Theme `multi_format`: epub, djvu, cbz, cbr, mobi, chm, xps within 100 characters of a reader name. Spot-check: about 9 in 10.
- "In addition to PDF, it can open EPub, DjVU, JPEG, PNG, GIF, Tiff, WebP, CBR, CBZ, DVI, XPS, ODT and other formats." ([38000153](https://news.ycombinator.com/item?id=38000153), 2023-10-24; about Okular)
- "The fact that it can display all these other formats as well is a nice bonus." ([27589555](https://news.ycombinator.com/item?id=27589555), 2021-06-22; about SumatraPDF)

**8. Sync across devices.** Readers want their position, highlights and notes to follow them from desktop to tablet to phone.
Count: 159 comments, 2.3% → 2.4% → 1.9%. Theme `sync_devices`: sync, across devices, multiple devices near a reader name or "PDF". Spot-check: about 6 in 10.
- "If any FOSS solution exists on all these platforms in order to sync position progress I'm all ears." ([22173410](https://news.ycombinator.com/item?id=22173410), 2020-01-28)
- "The main feature for me in Zotero is sync'ing across devices." ([40316478](https://news.ycombinator.com/item?id=40316478), 2024-05-10)

**9. Accessibility.** Commenters want PDFs to scale, reflow and expose their text to screen readers. Most say PDFs and their viewers fall short.
Count: 127 comments, 1.6% → 1.5% → 2.2%. Theme `accessibility`: screen reader, accessibility, visually impaired, text-to-speech, read aloud, tagged PDF. Spot-check: about 6 in 10.
- "PDF is very hard to impossible to scale for people who need big writing, due to reflowing being unavailable or not working properly." ([25939847](https://news.ycombinator.com/item?id=25939847), 2021-01-28)
- "PDF.js is the only PDF renderer to have a standard acessibility tree." ([31268595](https://news.ycombinator.com/item?id=31268595), 2022-05-05)
- "HTML-files/Webpages are accessible to screen readers and my browser's extensions work on them, extensions don't work on PDF files" ([42481906](https://news.ycombinator.com/item?id=42481906), 2024-12-21)

**10. Dark mode for the page.** Readers want the page itself dark, with images and figures kept in their real colours.
Count: 90 comments, rising: 0.9% → 1.2% → 1.5%. Theme `dark_mode`: dark mode, night mode, invert colours, recolour within 150 characters of a reader name or "PDF". Spot-check: about 8 in 10. Stories with "dark mode" or "invert" and "PDF" in the title rose from 1 to 4 to 10 (`extras.py`).
- "What I miss the most when reading a PDF file is the ability to use dark mode." ([23557662](https://news.ycombinator.com/item?id=23557662), 2020-06-17)
- "Automatically applying dark mode to documents tends to have poor results, especially when images are involved" ([39777221](https://news.ycombinator.com/item?id=39777221), 2024-03-21)
- "(people really want the dark theme)" ([31566526](https://news.ycombinator.com/item?id=31566526), 2022-05-31; SumatraPDF's author on his feature-request board)

**11. Clean text selection and copying.** Readers want to select a column or paragraph and paste clean text, ligatures included.
Count: 104 comments, 1.2% → 1.6% → 1.4%. Theme `text_select_copy`: select text, copy text, copy-paste, ligature near a reader name or "PDF". Spot-check: about 7 in 10.
- "Okular saved me huge amount of time because it allows to select a column in a PDF file and copy it to clipboard. All the other PDF viewers I tried would only select text sequentially line by line." ([31298594](https://news.ycombinator.com/item?id=31298594), 2022-05-07)
- "I hate these franken pdf viewers that make it impossible to copy text on mobile." ([34671207](https://news.ycombinator.com/item?id=34671207), 2023-02-05)

**12. Keyboard and vim navigation.** A vocal group of developers wants every action on the keyboard, often with vim keys.
Count: 92 comments, 1.3% → 1.3% → 1.2%. Theme `keyboard_vim`: vim keys or bindings, hjkl, keybindings, keyboard shortcuts, hotkeys, Vimium, Tridactyl near a reader name. Spot-check: about 6 in 10.
- "I’ve often yearned for something that uses vim keyboard commands and allows some keyboard based tagging and also search functionality." ([45600270](https://news.ycombinator.com/item?id=45600270), 2025-10-16)
- "It supports vim like marks, and powerful movements to manage a lot of pages in a reasonably sane way." ([23535246](https://news.ycombinator.com/item?id=23535246), 2020-06-16; about Zathura)
- "why the hell don't the vim keys (hjkl) smooth scroll?" ([24559348](https://news.ycombinator.com/item?id=24559348), 2020-09-22; to a pdf.js developer)

**13. Huge files that stay usable.** People want manuals, textbooks and datasheets of 500 to 6,000 pages to open, scroll and search without stalling.
Count: 91 comments, 1.1% → 1.2% → 1.4%. Theme `huge_files`: page counts of three digits or more, "large/huge/complex PDF" near a reader name. Spot-check: about 7 in 10.
- "It's one of the only windows apps that can handle textbook-sized pdfs well. I can search on text quickly too, while Adobe Reader can barely open any PDF larger than 100 pages." ([27972648](https://news.ycombinator.com/item?id=27972648), 2021-07-27)
- "You guys must have never had to open a 500+ page document, because for me, that's an all-but guaranteed way to make Preview crash." ([45599910](https://news.ycombinator.com/item?id=45599910), 2025-10-16)
- "For datasheets Edge is a parody. i need a TOC in a 6000 pages document, damn it." ([45631213](https://news.ycombinator.com/item?id=45631213), 2025-10-18)

**14. Search with all matches in view.** Readers want fast search and a panel that lists every match with context.
Count: 87 comments, 1.2% → 1.1% → 1.3%. Theme `search`: search in or through a document, search results, search panel or sidebar, find bar, Ctrl+F, fast or slow search. Spot-check: about 5 in 10.
- "there's no widget to show all search matches, afaik, and searching can be slow on really big files with thousands of pages." ([37993890](https://news.ycombinator.com/item?id=37993890), 2023-10-24; about SumatraPDF)
- "The evince feature I can't live without is the find, which shows a side panel with all matches in the document along with a bit of context." ([37995540](https://news.ycombinator.com/item?id=37995540), 2023-10-24)

**15. Page layouts.** Readers want two-page spreads, continuous scrolling and zoomed-out grids of many pages.
Count: 70 comments, 1.1% → 1.1% → 0.7%. Theme `page_layout`: two-page, side by side, facing pages, book view, continuous scroll, fit page or width. Spot-check: about 6 in 10.
- "Seeing two pages side by side, like a book, even on small screens makes a huge difference for me." ([13590994](https://news.ycombinator.com/item?id=13590994), 2017-02-07)
- "being able to quickly zoom out to see e.g. 30 pages (sometimes all the pages of a journal article) at the same time can be very useful." ([37995010](https://news.ycombinator.com/item?id=37995010), 2023-10-24)
- "I use edge only as double page endless scrolling pdf reader. Haven't found the alternative yet" ([35211117](https://news.ycombinator.com/item?id=35211117), 2023-03-18)

**16. Citations and link previews.** Readers of papers want to jump to a citation, preview it on hover and jump back.
Count: 66 comments, rising from 0.3% to 1.1% and 1.0%. Theme `citations_links`: clickable or internal links, jump back, back button, link preview, hover over, citation pop-ups near a reader name or "paper". Spot-check: about 8 in 10.
- "clicking on a citation in Acrobat reader is very frustrating." ([39770052](https://news.ycombinator.com/item?id=39770052), 2024-03-20)
- "if I mouseover an internal link in a pdf, it shows a little preview of where that link would take me" ([39809245](https://news.ycombinator.com/item?id=39809245), 2024-03-24; about Evince)
- "the fact that internal links in PDFs cannot be traversed makes it a non-starter for me as a technical document reader." ([25449884](https://news.ycombinator.com/item?id=25449884), 2020-12-16; about reMarkable 2)

**17. The outline in a sidebar.** Readers want the document's table of contents beside the page, and some want one generated when the file lacks it.
Count: 58 comments, 0.7% → 0.9% → 0.7%. Theme `toc_outline`: table of contents, TOC, outline, bookmarks panel, thumbnails near a reader name. Spot-check: about 6 in 10.
- "There is a TOC with the PDF and Chrome's PDF reader shows it in the sidebar. Other PDF readers should do the same." ([31125232](https://news.ycombinator.com/item?id=31125232), 2022-04-22)
- "Does the table of contents work if the pdf doesn't come with a table of contents? (That's something I always wished my pdf reader could do)" ([40313446](https://news.ycombinator.com/item?id=40313446), 2024-05-09)

**18. Reopen at the last page.** Readers want each file to reopen where they left it, and the open files to come back after a restart.
Count: 47 comments, 1.1% → 0.7% → 0.4%. Theme `remember_position`: remembers position or page, where I left off, reading position, saves your place. Spot-check: about 9 in 10.
- "when you re-open a PDF, opens it at the position you were last time(!!!)." ([14839643](https://news.ycombinator.com/item?id=14839643), 2017-07-24; about SumatraPDF)
- "That not remembering the last page you were at in Books drives me crazy." ([25448417](https://news.ycombinator.com/item?id=25448417), 2020-12-16)
- "(Okular is probably the best, but it can’t remember opened tabs.)" ([37994382](https://news.ycombinator.com/item?id=37994382), 2023-10-24)

**19. Auto-reload without locking the file.** People who write LaTeX or Typst want the viewer to reload when the file changes and never lock it.
Count: 27 comments, 0.3% → 0.4% → 0.4%. Theme `auto_reload_latex`: auto-reload, reloads on change, locks the file, SyncTeX, vimtex. Spot-check: about 8 in 10. Small, but in the SumatraPDF thread it was the most praised feature after speed.
- "It auto-reloads PDFs like Sumatra (IMO this is what draws the line between good and bad PDF readers, haha)." ([37994438](https://news.ycombinator.com/item?id=37994438), 2023-10-24; about Evince)
- "Exports/compiles failing, just because the PDF is still open somewhere is just unbelievably annoying." ([37995824](https://news.ycombinator.com/item?id=37995824), 2023-10-24)

Smaller wants, each under 25 comments: tabs (23), smooth zoom and scroll (23) and a portable build that needs no install (10).
- "Dark mode and tab switching with the keyboard are 2 great additions to the best lightweight PDF reader EVER!!!" ([37994982](https://news.ycombinator.com/item?id=37994982), 2023-10-24)
- "when zooming in quickly, it felt sluggish and hard to keep the zoom focus in the right place." ([44901801](https://news.ycombinator.com/item?id=44901801), 2025-08-14; about pdf.js)

## 2. What they complain about most

**1. Readers as a security hole.** Commenters see PDF readers, Acrobat above all, as a common way into a machine. They prefer viewers that run sandboxed, in the browser or in a VM.
Count: 370 comments, steady at 4.8% → 5.2% → 5.1%. Theme `security`: exploit, vulnerability, CVE, RCE, malware, sandbox, attack surface, zero-day near a reader name or "PDF". Spot-check: about 8 in 10. The top story in this area is "Security Flaws in Adobe Acrobat Reader Allow Gaining Root on macOS Silently" (871 points, 2020-05-14, [23180975](https://news.ycombinator.com/item?id=23180975)).
- "At this point Adobe have to be responsible for some overwhelming fraction of all desktop exploits." ([23181940](https://news.ycombinator.com/item?id=23181940), 2020-05-14)
- "Having a sandboxed PDF viewer that works 95% of the time is great." ([24555270](https://news.ycombinator.com/item?id=24555270), 2020-09-22)
- "Almost all exploits are in the boring 2D text/graphics/images parts." ([50220864](https://news.ycombinator.com/item?id=50220864), 2026-10-24)

**2. Slow and bloated software.** Acrobat draws most complaints about slow starts and huge installs. Preview, pdf.js and Okular draw some.
Count: 260 comments, 4.3% → 3.2% → 3.6%. Theme `slow_bloat`: slow, sluggish, bloated, takes forever, resource hog, sizes in MB or GB within 120 characters of a reader name. Spot-check: about 7 in 10.
- "I like my reader to simply be a reader instead of a 13GB bloated piece of garbage." ([45604604](https://news.ycombinator.com/item?id=45604604), 2025-10-16)
- "Adobe Acrobat Reader takes 3-digit MB to offer same feature & 5x more time to start." ([22021788](https://news.ycombinator.com/item?id=22021788), 2020-01-11)
- "It's just slow to start. Like very, very, sluggish slow. And it's very visible to users." ([37997238](https://news.ycombinator.com/item?id=37997238), 2023-10-24; SumatraPDF's author on Acrobat)

**3. Crashes and memory.** Preview crashes on large files and during edits. Browser viewers use gigabytes of memory on complex files.
Count: 83 comments, 1.3% → 1.0% → 1.2%. Theme `crash_memory`: crash, hang, freeze, beach ball, memory use, gigabytes of RAM near a reader name. Spot-check: about 6 in 10.
- "open - edit - reload - edit - reload - edit - Preview crashes - reopen - edit - reloads - edit - Preview crashes etc." ([37994160](https://news.ycombinator.com/item?id=37994160), 2023-10-24)
- "Preview has become pretty much unusable. It crashes often when zooming PDFs." ([15729860](https://news.ycombinator.com/item?id=15729860), 2017-11-18)
- "They'll gobble up 4GB of RAM like it's nothing" ([18672528](https://news.ycombinator.com/item?id=18672528), 2018-12-13; about browser viewers on big files)

**4. Rendering faults.** People notice blurry text, bad kerning, missing images and blurry printouts.
Count: 70 comments, 1.5% → 0.9% → 0.7%. Theme `render_fidelity`: blurry, kerning, garbled, renders wrong, missing fonts or images, rendering bugs near a reader name or "PDF". Spot-check: about 6 in 10.
- "Because PDFs in Preview are blurry on external screens ever since Sierra." ([23182452](https://news.ycombinator.com/item?id=23182452), 2020-05-14)
- "They can't print out pages which aren't blurry from the viewer even though they've had 8 years notice" ([28882867](https://news.ycombinator.com/item?id=28882867), 2021-10-15; about Firefox)
- "I opened an old PDF file on MacOS Monterey and found that Preview couldn't display the images in it." ([45605975](https://news.ycombinator.com/item?id=45605975), 2025-10-16)

**5. Upsells, subscriptions and sign-in walls.** Acrobat asks for an account or a subscription for tasks that used to be free, and other readers push paid tiers.
Count: 66 comments, rising: 0.4% → 0.9% → 1.2%. Theme `ads_upsell_sub_signin`: subscription, upsell, nag, ads in the app, sign in, Adobe ID, paywall, pay to edit or rotate within 120 characters of Acrobat, Adobe, Foxit or another reader. Spot-check: about 8 in 10. In the full comment set, Acrobat within 150 characters of sign-in, subscription or upsell rose from 4 to 16 to 15 comments (`extras.py`). "Adobe now requires you to purchase a subscription to rotate PDF pages" drew 208 points ([33709255](https://news.ycombinator.com/item?id=33709255), 2022-11-22).
- "And it recently asked me to sign in an account and give Adobe money to open a PDF??" ([45599763](https://news.ycombinator.com/item?id=45599763), 2025-10-15)
- "I swear I used to be able to do this in Adobe Acrobat without a paid subscription. Now I can't. And I don't mean editing, I mean merely filling them out, too." ([35194254](https://news.ycombinator.com/item?id=35194254), 2023-03-17)
- "No upsells like acrobat/foxit, better compatibility than pdf.js like your browser." ([31317642](https://news.ycombinator.com/item?id=31317642), 2022-05-09; recommending SumatraPDF)

**6. Documents that only work in Acrobat.** Government and bank forms built with XFA or scripts force people back to Acrobat.
Count: 62 comments, 0.4% → 1.1% → 0.8%. Theme `acrobat_only_docs`: only works or opens in Acrobat, requires or had to install Acrobat, XFA, dynamic forms, "Please wait...". Spot-check: about 9 in 10.
- "Plenty of rich PDF forms, including those used by the US government, require the use of "real" Adobe Reader to work correctly." ([13395312](https://news.ycombinator.com/item?id=13395312), 2017-01-13)
- "PDF is no longer a single format. You have to specify what type of PDF. Many will not open in anything but Acrobat." ([45599229](https://news.ycombinator.com/item?id=45599229), 2025-10-15)
- "there were official federal and state government PDFs we had to work with that only displayed correctly in Adobe Reader." ([45015114](https://news.ycombinator.com/item?id=45015114), 2025-08-25)

**7. Telemetry and uploads.** People distrust readers that phone home or send documents to a server.
Count: 42 comments, 0.4% → 0.5% → 0.8%. Theme `telemetry_privacy`: telemetry, phones home, spyware, tracking, analytics, uploads your files, runs locally near a reader name or "PDF". Spot-check: about 6 in 10. Adobe's Liquid Mode thread raised the same worry about "Processing in Document Cloud" ([24569697](https://news.ycombinator.com/item?id=24569697), 2020-09-23).
- "I’d like to be able to read PDF’s without third party connections to multiple sites, which Polar seems to do." ([20017649](https://news.ycombinator.com/item?id=20017649), 2019-05-26)
- "I got burned by Foxit when they started shipping spyware with it, so I'm hesitant now to try anything else." ([37995648](https://news.ycombinator.com/item?id=37995648), 2023-10-24)

**8. Files damaged on save.** Damage to text layers on save is the main fear about editing in a reader, and the Big Sur Preview thread is where it came to a head.
Count: 35 comments, 0.3% → 0.6% → 0.4%. Theme `save_damage`: corrupt, destroy, damage, mangle, strip the OCR or text layer, scrambled near "save" or a reader name. Spot-check: about 6 in 10. "Preview in macOS Big Sur is destroying PDFs" drew 359 points and 317 comments ([25447830](https://news.ycombinator.com/item?id=25447830), 2020-12-16).
- "After applying a signature to a PDF in Preview, the searchable text layer becomes scrambled so that numbers are no longer searchable or copy-able." ([38775028](https://news.ycombinator.com/item?id=38775028), 2023-12-26)
- "If you are going to write a PDF reader then you are going to have to accept those documents and not corrupt them." ([25449018](https://news.ycombinator.com/item?id=25449018), 2020-12-16)
- "I definitely blame Apple for destroying the PDF." ([25448865](https://news.ycombinator.com/item?id=25448865), 2020-12-16)

**9. AI features nobody asked for.** Since 2023, Acrobat's AI assistant buttons and pop-ups have drawn anger. This complaint is new and small, but it grew fast.
Count: 26 comments, all in 2023–26 (0 → 0 → 0.9%). Theme `ai_in_reader`: AI assistant, AI toolbar, AI features, Copilot, chat with PDF, summarise the PDF near a reader name. Spot-check: about 6 in 10; the rest are AI-reader pitches. In the full comment set, Acrobat within 200 characters of an AI term appears 0, 2 and 17 times (`extras.py`).
- "I have ZERO idea why anyone would want AI inside a program used to view documents." ([49344941](https://news.ycombinator.com/item?id=49344941), 2026-08-18)
- "I counted no fewer than six buttons for the AI assistant but couldn't figure out how to see chapters for the document I was reading." ([45604361](https://news.ycombinator.com/item?id=45604361), 2025-10-16)
- "A PDF reader should read PDFs, there's no need for it to have its own (possibly paid) cloud service and an AI chat bot." ([45413168](https://news.ycombinator.com/item?id=45413168), 2025-09-29)

**10. Printing.** People keep Acrobat for its print dialog, and canvas-based viewers print blurry bitmaps.
Count: 22 comments, 0.3% → 0.2% → 0.4%. Theme `print_problems`: printing fails, print dialog, print quality, can't print, prints blurry near a reader name. Spot-check: about 7 in 10.
- "What I miss from Acrobat is its Print dialog. Yes, really, the Print dialog." ([45015124](https://news.ycombinator.com/item?id=45015124), 2025-08-25)
- "it will just print bitmaps which leads to blurry print or OOM errors if doing high dpi." ([44131484](https://news.ycombinator.com/item?id=44131484), 2025-05-29; about pdf.js and a PDFium-WASM viewer)
- "Did they ever fix the issues around printing? I always ended up using a different PDF viewer just because of that." ([37996294](https://news.ycombinator.com/item?id=37996294), 2023-10-24; about SumatraPDF)

## 3. What excites them

Points come from `data/stories.jsonl`. Six groups of stories drew attention.

**Browser viewers gaining real features.** The highest-scoring reader story of the decade is "Embedded PDF viewer in Firefox 81 supports filling forms" (1,093 points, 375 comments, 2020-09-22, [24554581](https://news.ycombinator.com/item?id=24554581)). Follow-ups did well too: "Implementing form filling and accessibility in the Firefox PDF viewer" (177 points, 2021-10-15, [28880936](https://news.ycombinator.com/item?id=28880936)) and "Edit PDFs for Free with Firefox PDF Editor" (112 points, 2024-05-16, [40383879](https://news.ycombinator.com/item?id=40383879)). One reply sums up the appeal: "I don't want to start explaining to my mother, over the phone, how to install and use the pdf viewer anymore :|" ([24554720](https://news.ycombinator.com/item?id=24554720), 2020-09-22).

**SumatraPDF as proof that small and fast is possible.** "SumatraPDF Reader" (586 points, 2023-10-24, [37993575](https://news.ycombinator.com/item?id=37993575)), "Lessons learned from 15 years of SumatraPDF" (350 points in 2021, [27968900](https://news.ycombinator.com/item?id=27968900); 585 points on a 2023 repost, [35065785](https://news.ycombinator.com/item?id=35065785)) and "SumatraPDF 3.4 Released" (277 points, 2022-05-31, [31566401](https://news.ycombinator.com/item?id=31566401)). The comments praise speed, the small install and the absence of upsells.

**Readers built for papers and textbooks.** "Google Scholar PDF Reader" drew 422 points (2024-03-20, [39768438](https://news.ycombinator.com/item?id=39768438)) for clickable citations. Sioyek, a viewer with reference previews, a generated outline and vim keys, drew 272 points (2021-07-20, [27893303](https://news.ycombinator.com/item?id=27893303)), 238 points (2022-02-19, [30395649](https://news.ycombinator.com/item?id=30395649)) and 336 points (2024-05-09, [40313143](https://news.ycombinator.com/item?id=40313143)). A Sioyek user put it this way: "My favorite feature from this program that doesn't exist in any other PDF reader  (AFAIK) is visual mark mode, where it highlight each line you read." ([40313622](https://news.ycombinator.com/item?id=40313622), 2024-05-09). Okular got two strong threads (172 points, 2019-03-24, [19478623](https://news.ycombinator.com/item?id=19478623); 221 points, 2022-05-07, [31294580](https://news.ycombinator.com/item?id=31294580)).

**Fast viewers in the browser through WebAssembly.** "MuPDF WASM Viewer Demo" drew 318 points (2024-04-20, [40096113](https://news.ycombinator.com/item?id=40096113)): "The performance is astonishing. On my underpowered android, the PDF was super smooth." ([40096303](https://news.ycombinator.com/item?id=40096303)). EmbedPDF, an MIT viewer on PDFium compiled to WebAssembly, drew 357 points as "Show HN: I built a free alternative to Adobe Acrobat PDF viewer" (2025-08-14, [44901683](https://news.ycombinator.com/item?id=44901683)) after a quieter first post (27 points, 2025-05-29, [44126177](https://news.ycombinator.com/item?id=44126177)). One reply: "Unlike Acrobat Reader, this actually made me want to read a pdf." ([44907230](https://news.ycombinator.com/item?id=44907230)). Earlier, "Pdf.js Express" drew 334 points (2020-04-02, [22763656](https://news.ycombinator.com/item?id=22763656)).

**Reflow and dark-mode tools.** "Show HN: PDFlower – Reflow PDF papers for small-screen reading" drew 216 points (2017-01-04, [13318289](https://news.ycombinator.com/item?id=13318289)). Adobe's Liquid Mode drew 132 points (2020-09-23, [24568695](https://news.ycombinator.com/item?id=24568695)); phone readers liked it, and others worried about cloud processing. "Show HN: Veil – Dark mode PDFs without destroying images, runs in the browser" drew 100 points (2026-03-26, [47529306](https://news.ycombinator.com/item?id=47529306)): "Most dark mode tools break images, so this solves a real problem." ([47541751](https://news.ycombinator.com/item?id=47541751)). Most other reflow and dark-mode tools drew under 25 points each, but they keep coming: 27 reflow stories and 15 dark-mode stories over the decade (`extras.py`).

**Local and safe handling.** "Dangerzone: Convert potentially dangerous PDFs…" (317 points, 2020-03-08, [22516496](https://news.ycombinator.com/item?id=22516496)) and "Convert potentially dangerous PDFs to safe PDFs" (181 points, 2026-01-21, [46712815](https://news.ycombinator.com/item?id=46712815)). "PaperPilot – Zero-Cloud, Pure-Rust PDF Engine with WASM and MCP" drew 236 points (2026-10-26, [50238739](https://news.ycombinator.com/item?id=50238739)). Its comments liked the local-only design but doubted an AI-written engine: "I stay away from things like lopdf because of how they fail to handle broken pdfs." ([50242641](https://news.ycombinator.com/item?id=50242641)).

AI readers did not draw the same interest. Of 27 "Show HN" reader or viewer stories in 2023–26, the 11 with AI, chat or MCP in the title had a median of 2 points and a maximum of 8. The other 16 had a median of 4 and a maximum of 357 (`extras.py`).

## 4. Tools and sentiment over time

Mentions per period (`tools.py`; one count per comment):

| Reader | 2016–19 | 2020–22 | 2023–26 |
|---|---:|---:|---:|
| Adobe Acrobat / Reader | 177 | 373 | 341 |
| pdf.js / Firefox viewer | 152 | 235 | 264 |
| SumatraPDF | 48 | 183 | 246 |
| macOS Preview | 81 | 226 | 154 |
| Okular | 101 | 190 | 129 |
| reMarkable | 21 | 145 | 72 |
| Evince | 92 | 80 | 59 |
| Foxit | 55 | 86 | 72 |
| Chrome / Edge viewer, PDFium | 56 | 60 | 69 |
| Zathura | 39 | 69 | 47 |
| MuPDF | 34 | 47 | 59 |
| KOReader | 8 | 35 | 36 |
| Zotero (reader) | 16 | 30 | 29 |
| PDF Expert | 9 | 30 | 13 |
| Sioyek | 0 | 21 | 24 |

For the five most-discussed readers, I labelled 20 random comments per period by hand (`sentiment_labels.json`). P is positive, N negative, R negative but needs it for a task, O neutral or no opinion.

| Reader | 2016–19 | 2020–22 | 2023–26 |
|---|---|---|---|
| Acrobat / Reader | P5 N6 R2 O7 | P4 N7 R3 O6 | P1 N9 R2 O8 |
| macOS Preview | P6 N6 O8 | P8 N3 O9 | P7 N4 O9 |
| SumatraPDF | P15 N3 O2 | P14 N0 O6 | P11 N2 O7 |
| pdf.js / Firefox | P3 N4 O13 | P6 N6 O8 | P6 N6 O8 |
| Chrome / Edge, PDFium | P7 N4 O9 | P6 N5 O9 | P6 N2 O12 |

**Adobe Acrobat / Reader: from bloated necessity to intrusive.** Acrobat is the most-mentioned reader in every period, and opinion fell. In 2016–19 some still praised its tolerance of broken files and its Android reading mode, but most called it slow and insecure and kept it only for forms. By 2023–26, one of 20 sampled comments was positive. The new complaints are sign-in, subscriptions and AI prompts (section 2, items 5 and 9). A few still concede its reach: "I hate Acrobat but it is still the best pdf program on Windows." ([45631155](https://news.ycombinator.com/item?id=45631155), 2025-10-18). The common view is harsher: "They designed the PDF format but still manage to have the worst PDF reader on the entire market." ([45599423](https://news.ycombinator.com/item?id=45599423), 2025-10-15).

**macOS Preview: loved, with regressions.** Mac users call Preview the best simple viewer: "Preview is easily the best simple PDF viewer I've ever seen." ([19561562](https://news.ycombinator.com/item?id=19561562), 2019-04-03). Each period also has a regression: blurry rendering after Sierra (2017–19), saves that damaged OCR text in Big Sur (2020), and crashes on 500-page files. Opinion rose after 2019 and held. Some praise is relative: "Preview is only best because the major alternatives (Acrobat) suck more." ([45599910](https://news.ycombinator.com/item?id=45599910), 2025-10-16).

**SumatraPDF: the most-loved reader of the decade.** Mentions grew fivefold, from 48 to 246, and 40 of 60 sampled comments were positive. People praise its start-up speed, small size, saved position, auto-reload and lack of upsells: "SumatraPDF singlehandedly makes the format readable for me." ([32965842](https://news.ycombinator.com/item?id=32965842), 2022-09-24). The limits are Windows only, no editing or signing, and plain defaults: "It's strictly a viewer only, so it doesn't even have rudimentary annotations like highlights or underlines." ([31300292](https://news.ycombinator.com/item?id=31300292), 2022-05-08). A reply noted that SumatraPDF added basic annotations in version 3.3 ([31304484](https://news.ycombinator.com/item?id=31304484)).

**pdf.js / Firefox: from slow afterthought to safe default, still split.** In 2017 a commenter wrote: "PDF.js is slow, buggy, and can’t edit documents." ([15941646](https://news.ycombinator.com/item?id=15941646), 2017-12-16). Forms (2020), XFA (2021), annotations (2022) and editing (2023–24) moved the sample from 3 positive to 6. Its security case stayed constant: "It starts to look like Mozilla's pdf.js is the most secure viewer, at least it's using the browser's sandbox" ([23185427](https://news.ycombinator.com/item?id=23185427), 2020-05-14). Speed on big files did not improve in commenters' eyes: "pdf.js is a parody if you have more than 3 pages." ([45631155](https://news.ycombinator.com/item?id=45631155), 2025-10-18). A 2024 bug that let a PDF run JavaScript hurt it as an embedded library (CVE-2024-4367, 207 points, [40414718](https://news.ycombinator.com/item?id=40414718)). Some now prefer it to a desktop reader: "I prefer the Firefox PDF viewer personally, and now that it can do all this I uninstalled the default Ubuntu PDF reader recently." ([48980816](https://news.ycombinator.com/item?id=48980816), 2026-07-20).

**Chrome and Edge (PDFium): fast and fine, then Edge took Adobe's engine.** Commenters rate PDFium faster than pdf.js: "PDF.js doesn't come close to performance of native C++ code of pdfium (our tests showed 3-10x slowdown)." ([15169271](https://news.ycombinator.com/item?id=15169271), 2017-09-04). Edge's viewer won praise as Windows' default: "It loads quickly, doesn't try to sell me things, and stays out of the way." ([34831056](https://news.ycombinator.com/item?id=34831056), 2023-02-17). Microsoft's 2023 move to Acrobat's engine worried users: "now it’ll harass you to do ‘more’ with Adobe junk in it." ([34720574](https://news.ycombinator.com/item?id=34720574), 2023-02-09). All browser viewers share one complaint: "any PDF viewer that runs in a browser just can't handle big documents" ([33995285](https://news.ycombinator.com/item?id=33995285), 2022-12-15).

**Linux readers: capable, but with poor defaults.** Okular leads (forms, annotations, column selection, Windows builds), and Evince and Zathura with MuPDF follow. Their weak point is defaults and polish: "they suffer from ugly defaults." ([41002383](https://news.ycombinator.com/item?id=41002383), 2024-07-19). Some users are content: "I’m convinced it is impossible to do better than Evince" ([45599819](https://news.ycombinator.com/item?id=45599819), 2025-10-15). Evince's mentions fell from 92 to 59, while MuPDF's rose from 34 to 59, mostly for speed.

**Tablets and research readers.** reMarkable mentions peaked in 2020–22 (145). Owners praise pen notes, while PDF-heavy readers report "PDF support is horrible, particularly if you have a two column layout file." ([23539124](https://news.ycombinator.com/item?id=23539124), 2020-06-16). On the iPad, PDF Expert, GoodNotes and GoodReader win praise. Sioyek and Zotero's reader grew among researchers.

## 5. How the decade changed

- **Acrobat lost its remaining goodwill.** Its positive share in the sample fell from 5 in 20 to 1 in 20, while its negative share rose from 6 to 9. Acrobat near sign-in, subscription or upsell went from 4 to 16 to 15 comments in the full set, and Acrobat near AI from 0 to 2 to 17.
- **The browser viewer became the safe default for casual reading.** Firefox added forms, annotations and editing between 2020 and 2024, and Edge became Windows' default viewer. Browser viewers appear in 11–13% of reader comments in every period. Heavy readers still want a dedicated app for big files, printing and speed.
- **Dark mode moved from a terminal trick to a common request.** Its comment share rose from 0.9% to 1.2% to 1.5%, and dark-mode stories rose from 1 to 4 to 10. Preview added a dark-page toggle in 2025 (7 points, [44313468](https://news.ycombinator.com/item?id=44313468)).
- **Reading papers became its own use case.** Citation and link-preview comments rose from 0.3% to about 1%. Sioyek, Zotero's reader and Google Scholar's reader (422 points) all appeared in this decade.
- **Talk of tablets and annotation peaked in 2020–22.** Tablet comments fell from 16.0% to 11.8% and annotation comments from 10.6% to 7.5% in 2023–26.
- **AI reached readers, mostly as something to dismiss.** AI-in-reader comments went from none to 26 in 2023–26, nearly all hostile to Acrobat's assistant. AI reader Show HNs drew a median of 2 points.

## 6. What this means for a PDF reader built today

1. **Make start-up and first-page time a release gate.** Speed is the most common praise and slowness the second most common complaint (470 comments together). Every loved reader, from SumatraPDF and MuPDF to Zathura, is loved for it. Lamina's viewer performance harness already measures first page warm and cold; its numbers should be budgets that fail a build, as the WASM size budgets do.
2. **Keep the reader a reader.** Upsells, sign-in walls and AI pop-ups turned Acrobat's sample from 5 positive to 1 (66 and 26 comments, both rising). Edge's praise was that it "doesn't try to sell me things". ADR 0011 already requires opt-in telemetry and hosted features. Any AI or cloud feature should stay off until the user turns it on, and should never block reading.
3. **Treat every file as hostile, and say so.** Security is the largest complaint (370 comments). Commenters trust viewers that run in the browser sandbox. Lamina's design fits this: PDFium in a document worker, documents never sent to a server, and a fuzzed core. The privacy and sandbox guarantees belong on the website, where users can read them.
4. **Stay fast on huge and broken files.** People test readers on 500- to 6,000-page manuals and on broken PDFs, and they abandon those that crash or stall (91 huge-file and 83 crash comments). Tiled rendering, the bitmap cache and ADR 0003's tolerant reading suit this. Lamina's tests should include 1,000-page datasheets and textbooks, as well as damaged files.
5. **Save without damage, and write annotations every viewer can show.** Annotation is the second-largest want (679 comments), and Preview's damaged saves drew 359 points. The save planner's incremental writes, which keep the original bytes, answer this fear directly. Annotations need to open correctly in Acrobat, Preview and pdf.js.
6. **Offer a reading mode and a dark page that keeps images.** Reflow comes up in 501 comments and dark mode in 90, a rising share, yet no reader has solved either. Veil (100 points) won praise for one detail: images keep their colours. Lamina's extraction already returns blocks in reading order, which could feed a one-column reading mode. Its page-appearance setting, separate from the UI theme, is the right place for a dark page.
7. **Serve paper and manual readers.** These readers want a search panel with every match, an outline sidebar, link previews with a way back, and to reopen at the last page. They also want auto-reload without file locks and full keyboard control. Each is small (27 to 92 comments), but together they explain Sioyek's three popular launches and Google Scholar's reader (422 points).
8. **Fill AcroForms well and name XFA plainly.** Forms and signatures are why people keep Acrobat (283 comments). XFA-only government forms are the most cited dead end (62 comments). Filling and signing AcroForms in the reader removes the most common reason to install Acrobat. When a file needs XFA, the reader should name the problem instead of showing "Please wait...".

## 7. Method and limits

**What I read.** I read a stratified random sample of 470 comments from `areas/readers.jsonl`: 20 from 2016 and 45 from each year from 2017 to 2026, with at most 3 per thread and a fixed seed. I also read about 680 comments from 23 threads: the main SumatraPDF, Acrobat, Preview Big Sur, Firefox 81, "How do you read long PDFs?", Google Scholar, Sioyek, Okular, MuPDF WASM, EmbedPDF, Edge/Acrobat, Liquid Mode, Veil and PaperPilot threads, among others. For each theme I then read 10 to 20 random matches, and for sentiment I labelled 300 comments.

**How I counted.** `tools/hn-research/counts/readers/themes.py` holds one regular expression per theme. Most themes also require a reader name or "PDF" within 120 to 150 characters of the match. Each comment counts once per theme, and the script reports counts by period (2016–19, 2020–22, 2023–26). I tightened a theme whenever its spot-check showed many off-topic matches. The spot-check shares in sections 1 and 2 are from the final versions and range from about 5 in 10 (search) to 9 in 10 (forms, reopening at the last page). `tools.py` counts reader mentions. `extras.py` counts the full deduplicated comment set (58,841 comments from `data/comment_hits.jsonl` and `data/threads.jsonl`) and story titles. [`tools/hn-research/verify_quotes.py`](../../../../tools/hn-research/verify_quotes.py) checks that each quote appears word for word in its comment and runs 40 words or fewer. All quotes passed. I dropped a word-list sentiment score because it rated Acrobat neutral, against what I read. I used the hand labels in `sentiment_labels.json` instead.

**Limits.**
- HN commenters are mostly developers. Linux users, vim users and academics are over-represented, so keyboard navigation, Zathura and LaTeX reload count for more here than among all users. Forms, signing and phone reading likely count for more among all users.
- The area file is a classifier's output. It includes comments that mention a PDF reader only in passing, which is why most themes require context nearby. The counts are comments, not people: one long thread can lift a period, as the 121-comment SumatraPDF thread lifts 2023.
- Comment texts are cut to 1,500 characters, so some later mentions are missed. The first period starts in November 2016, and the last ends in early November 2026.
- The regexes miss paraphrases and still admit some noise, so read the counts as relative sizes, not exact totals. One person (me) did the hand labels, with 20 comments per cell, so small differences between cells mean little.
- Some threads, such as "The Size of Adobe Reader Installers Through the Years", are missing from `stories.jsonl`. I give no points for them.
