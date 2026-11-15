# Competitive teardown and friction audit of PDF applications (late 2026)

> Method note: research done 2026-10-29 via web search. Direct page fetches to adobe.com and community.adobe.com were blocked by the session's network allow-list, so Adobe claims rely on search-result snippets of those pages plus third-party aggregators. **Many price figures come from aggregator or reseller sites (pricingsaas, casrai, costbench, temperstack, StackSocial) and conflict with each other. Verify them against vendor pricing pages before quoting.** Labels: **[Fact]** = vendor-documented or reported by more than one independent source; **[Report]** = an individual user or anecdote; **[Opinion]** = a reviewer's or analyst's judgement; **[Old]** = before 2024.

## 1. Pricing models and price points (2025–2026)

### Takeaway
The market splits into Adobe (subscription-only, with AI now bundled into a higher "Studio" tier), challengers that use perpetual licences as a selling point (Foxit, Xodo PDF Studio, PDF Expert lifetime), freemium web tools that ration tasks per day (Smallpdf, iLovePDF, Xodo Web), and free or open-source viewers (Sumatra, Okular, PDF.js, Stirling-PDF core). Stirling-PDF already gates SSO, audit logs and similar features behind a paid tier. Price confusion, including different prices per region and per platform, is itself a complaint.

### Cited Findings
- **Adobe Acrobat Studio** launched in 2025 Q3 at **$24.99/mo**. It bundles Acrobat Pro, AI Assistant, PDF Spaces and Adobe Express Premium. [Fact, aggregator] — [PricingSaaS](https://pricingsaas.com/companies/adobe.creativecloud/diffs/2025Q3); breakdown of the Studio plans — [Schneider licensing guide](https://www.schneider.im/adobe-acrobat-studio-licensing-guide-plans-features-pricing-comparison/)
- Acrobat Pro is about **$19.99/mo** on an annual plan billed monthly, Standard is about **$14.99/mo**, and Studio for teams is about **$29.99/user/mo**. The AI Assistant add-on is listed at $4.99/mo by one source and $1.99 by another (conflict, unverified). — [thewearify](https://thewearify.com/acrobat-professional-price/), [aiproductivity.ai](https://aiproductivity.ai/pricing/adobe-acrobat/), [design-offset](https://design-offset.com/adobe-acrobat-ai-assistant-plans-pricing/)
- Acrobat Standard had a price-increase announcement from a reseller. — [Schneider](https://www.schneider.im/lu/adobe-acrobat-standard-price-increase-announcement/)
- On mobile, the free Acrobat app covers viewing, search, comments, form filling and sharing. Paid mobile plans unlock editing and export, plus an AI Assistant add-on for Q&A and summaries. [Fact, Adobe help] — [Acrobat mobile FAQ](https://helpx.adobe.com/acrobat/mobile-app-faq.html)
- **Foxit** still sells a **perpetual** licence (PDF Editor 14, about **$209.99** one-time). PDF Editor+ subscription is about **$159.99/yr** and the base subscription about **$131.88/yr ($10.99/mo)**, per US pricing checked Aug 2026. Other aggregators give $159/$249 or $10.99/$13.99 (conflict). — [casrai price guide](https://web.archive.org/web/20260926000124/https://casrai.org/guides/foxit-pdf-editor-price), [casrai subscription guide](https://web.archive.org/web/20260926000124/https://casrai.org/guides/foxit-pdf-editor-subscription), [aiproductivity.ai](https://aiproductivity.ai/pricing/foxit-pdf-editor/)
- Foxit's bundled AI credit allowance reportedly rose from 20 to 300 credits/month in late Aug 2026 (unconfirmed by Foxit). — [PricingSaaS diff 2026W37](https://pricingsaas.com/companies/foxit.pdfeditor/diffs/2026W37)
- **PDF Expert (Readdle)** is sold as a yearly subscription (about $6.67/mo billed yearly, with prices geolocated in EUR) and as a Mac **lifetime** licence, often discounted to **$69.97–$79.99** through StackSocial during 2026. The lifetime licence **excludes AI and translation, which need a separate subscription, and does not carry over to iPhone or iPad**. [Report, review summaries] — [Frontdesk Review](https://frontdeskreview.com/software/pdf-tools/pdf-expert.md), [Macworld deal](https://macworld.com/article/3217258/pay-once-skip-adobes-subscription-pdf-expert-for-mac-is-79-99-42-off-for-life.html), [toolradar reviews](https://toolradar.com/tools/pdf-expert/reviews)
- PDF Expert is rated 4.7/5 on G2 from 33 ratings (Aug 2026). — [toolradar](https://toolradar.com/tools/pdf-expert/pricing)
- **GoodNotes 6** has a free tier, **$9.99/yr**, or **$29.99 one-time**. GoodNotes 5 was a $9.99 one-time purchase. [Old: 2023 launch figures; current pricing not verified] — [Malay Mail](https://www.malaymail.com/news/tech-gadgets/2023/08/24/is-it-all-good-with-the-updated-goodnotes-6/87000), [Kimola App Store analysis](https://web.archive.org/web/20251209225112/https://kimola.com/reports/unlock-insights-goodnotes-6-customer-feedback-analysis-app-store-us-148345)
- **LiquidText**: listings show plans from $3.99/mo. An older listing shows a $29.99 standalone licence and $7.99/user/mo for cloud (Nov 2023, [Old]). G2 says pricing isn't published. — [GetApp IE](https://www.getapp.ie/software/2060201/liquidtext), [LawNext](https://directory.lawnext.com/products/liquidtext/pricing/), [G2](https://www.g2.com/products/liquidtext/pricing)
- **Xodo** (official blog, June 2026): Xodo Web is free with **1 file download/day** and a 3-day trial, then $7.99/mo yearly or $10.99/mo monthly. Xodo PDF Studio is $9.99/mo yearly, $19.99/mo monthly, or **$240 perpetual**. Xodo PDF Reader is a free desktop viewer for Windows, macOS and Linux. — [Xodo plans explained](https://xodo.com/blog/xodo-plans-explained)
- **Smallpdf**: the free tier is commonly reported as **2 tasks/day** with file-size limits, and Pro is about **$12/mo**. **iLovePDF** free-tier limits are reported inconsistently (3 tasks/hour, 1–2 tasks/hour, or 3/day), and premium is about $7/mo billed annually. Sources are mostly competitor comparison pages. — [codaone](https://www.codaone.ai/compare/ilovepdf-vs-smallpdf), [Capterra compare](https://capterra.com/compare/172606-173963/Smallpdf-vs-iLovePDF), [Capterra Smallpdf reviews](https://www.capterra.com/p/172606/Smallpdf/reviews/?page=5)
- **Stirling-PDF**: the core is open source, but some components are separately licensed and proprietary. The Team plan costs **$99/month per installation (100 users)**. Enterprise uses offline `.lic` keys and adds SSO, SAML and audit logs. Licences are bound to a machine fingerprint. — [Stirling docs: Paid Offerings](https://docs.stirlingpdf.com/Paid-Offerings/), [Modes and Licensing](https://docs.stirlingpdf.com/Modes%20and%20Licensing)
- **SumatraPDF, Okular, Firefox PDF.js, Chrome and Edge viewers, and macOS Preview** are free. Sumatra describes itself as "fast, small, packed with features, customizable, free". — [sumatrapdfreader.org](https://www.sumatrapdfreader.org/free-pdf-reader)

### Inferences
- "Pay once" is now a marketing hook in its own right. Macworld and Boing Boing ran repeated 2026 deal posts framing PDF Expert as "skip Adobe's subscription". That suggests strong user resentment of subscriptions that a free app can address directly.
- AI is becoming the reason vendors move users up a tier: Adobe's Studio, PDF Expert's AI add-on and Foxit's AI credits. For lamina, a stance of "no AI tax; core features never gated" is a clear way to stand apart. [Opinion]
- Web tools ration *tasks*, not features. A local-first web app (WASM) with no task caps undercuts their entire monetisation model.

### Gaps
- Adobe's own pricing page could not be fetched (network block), so the AI Assistant add-on price is unresolved ($1.99 vs $4.99).
- I found no verified current (2026) GoodNotes or LiquidText price sheet.

## 2. Common complaints per app (paywalls, bloat, cold start, crashes, sign-in, ads, cloud upsell)

### Takeaway
Acrobat attracts the heaviest criticism: about 2 GB footprint, slowness even on capable hardware, a disliked redesign, and sign-in walls on mobile. GoodNotes and LiquidText draw anger over moves to subscriptions and forced upgrades. Preview is loved for its simplicity but damages forms and other apps' annotations. Web tools frustrate users with task caps and mandatory uploads. Open-source tools (Okular, Stirling) are criticised for clunky UX rather than for paywalls.

### Cited Findings
- **Acrobat bloat** [Report, Nov 2025]: Windows lists Reader at **about 2.04 GB**, and a user says they "couldn't believe how bloated" it had become. Threads titled "Bloated, inefficient product, cancelling subscription" and "Poor performance on a capable system (i7, 16 GB RAM, SSD)" exist. — [Adobe Community: bloated](https://community.adobe.com/questions-9/bloated-inefficient-product-cancelling-subscription-1300380), [Adobe Community: poor performance](https://community.adobe.com/questions-9/poor-performance-on-a-capable-system-i7-16gb-ram-ssd-1305162)
- **Acrobat new UI** [Report, Oct 2025]: "the new UI is horrible and there are countless threads saying so". Adobe keeps a "Disable new Acrobat" toggle (Menu on Windows, View on macOS), and users say it helps. — [Adobe Community: new interface](https://community.adobe.com/questions-9/why-does-acrobat-have-a-new-interface-and-how-do-i-switch-back-to-the-classic-interface-1297830)
- **Acrobat slowness/hangs**: "incredibly slow & keeps hanging" threads run for several pages (2024–2025). There are slow cold-start reports on Windows 11, and macOS Ventura and Sonoma lag reports. Build 24.005.20320 (32-bit and 64-bit) is reported slow when Reader is already open (Jan 2025). — [Reader hanging](https://community.adobe.com/t5/acrobat-reader-discussions/adobe-acrobat-reader-is-incredibly-slow-amp-keeps-hanging/m-p/15124625), [slow start Win11](https://community.adobe.com/t5/acrobat-discussions/acrobat-very-slow-to-start-up-and-very-laggy-when-using-it-on-windows-11/m-p/15555450), [32-bit slow](https://community.adobe.com/t5/acrobat-reader-discussions/acrobat-reader-32-bit-version-24-005-20320-excessively-slow/td-p/15087496), [Ventura lag](https://community.adobe.com/t5/acrobat-reader-discussions/adobe-reader-lagging-slow-and-unusable-macos-ventura-13-2-1/m-p/15484402/highlight/true)
- **Acrobat mobile sign-in** [Fact, Adobe help]: sign-in is "required to access free features that leverage online services" and cloud save. A sign-in pop-up can be dismissed. — [Acrobat Android docs](https://www.adobe.com/devnet-docs/acrobat/android/en), [mobile FAQ](https://helpx.adobe.com/acrobat/mobile-app-faq.html)
- **SumatraPDF**: HN commenters (2023, [Old]) moved to it because of "Adobe's aggressive revenue pursuit". Its editing is limited, it is Windows-only, and users miss it on Mac and Linux. — [GeekNews summary of HN thread](https://news.hada.io/topic?id=11501), [AlternativeTo](https://alternativeto.net/software/sumatra-pdf/about)
- **Foxit**: Capterra reviews call the Reader "lightweight" and "cleaner". Trustpilot (company-wide) is 1.5/5 from 165 reviews, with themes not visible. Foxit had several 2025 security advisories, and in 2018 Cisco Talos disclosed 18 vulnerabilities in Foxit Reader ([Old]). — [Capterra](https://www.capterra.com/p/175376/Foxit-PDF-Reader/reviews/), [Trustpilot NL-BE](https://nl-be.trustpilot.com/review/www.foxitsoftware.com?page=5), [Canadian Cyber Centre AV25-623](https://cyber.gc.ca/en/alerts-advisories/foxit-security-advisory-av25-623), [Cisco Talos](https://blogs.cisco.com/security/talos/vulnerability-spotlight-multiple-issues-in-foxit-pdf-reader-2)
- **GoodNotes 6**: complaints include a near-nonexistent upgrade discount, nag screens pushing GoodNotes 5 users to subscribe, heavily promoted AI that was "poorly timed", and Apple-only lock-in for the one-time licence. — [Malay Mail](https://www.malaymail.com/news/tech-gadgets/2023/08/24/is-it-all-good-with-the-updated-goodnotes-6/87000) [Old], [GoodNotes feedback forum](https://feedback.goodnotes.com/forums/191274-customer-suggestions-for-goodnotes/suggestions/49324559-goodnotes-5-v-6), [Feedbagel](https://feedbagel.com/post/goodnotes-6-transition-to-subscription-model-frustrates-users-who-bought-lifetim), [brainbound](https://www.brainbound.blog/goodnotes-membership-cost-features); a r/GoodNotes thread "What happened to GoodNotes" (Nov 2024) — redlib mirror (offline, no archived copy, 2026-10-30)
- **LiquidText**: the subscription is described as "restrictive and costly, especially for students". A reviewer paid upfront and still needed a subscription for all features. Cloud sync costs extra. — [GetApp](https://www.getapp.com/collaboration-software/a/liquid-text/), Capterra via Gartner (offline, no archived copy, 2026-10-30), [softwarefinder](https://softwarefinder.com/legal/liquidtext/reviews?page=4)
- **macOS Preview**: it auto-saves PDFs on close without warning and can "mutilate" annotations made by other editors (Howard Oakley, Aug 2025). A 2025 commenter says Preview gives "a false sense that they are successfully filling forms" and will "EAT" JavaScript forms. Radio-button input is lost after reopening (macOS 12+). Preview has no digital (certificate) signatures. — [Eclectic Light: Inside PDF annotations](https://eclecticlight.co/2025/08/14/inside-pdf-annotations/), [Michael Tsai blog, Aug 2025](https://mjtsai.com/blog/?p=49000), [Apple dev forums](https://developer.apple.com/forums/thread/735073), [MacMost](https://macmost.com/common-mac-pdf-problems.html) [Old]; long-running PDFKit regression history — [MacStories](https://www.macstories.net/linked/developers-report-serious-pdf-bugs-in-macos-sierra/) [Old], [Michael Tsai 2023 "State of Mac PDF Software"](https://mjtsai.com/blog/2023/06/29/the-state-of-mac-pdf-software/) [Old]
- **Xodo**: the free version is praised, and one user mentions a "no ads policy" (unverified). It reportedly lags on low-end Windows machines. — [SoftwareSuggest](https://www.softwaresuggest.com/xodo), [Capterra](https://www.capterra.com/p/210439/Xodo/)
- **Smallpdf/iLovePDF**: daily task caps and file-size limits, and every file is uploaded to their servers (stated retention is 1–2 h). — [pdnob "Is Smallpdf safe"](https://www.pdnob.com/pdf-tips/is-smallpdf-safe.html), [codaone](https://www.codaone.ai/compare/ilovepdf-vs-smallpdf)
- **Stirling-PDF**: its interface is "functional rather than polished", and some features are Enterprise-only. Desktop mode without a server lacks OCR, conversion, compression and repair. It scores 3.7 on AlternativeTo (6 reviews). — [aicybr guide 2026](https://aicybr.com/blog/stirling-pdf-self-hosting-guide), [AlternativeTo](https://www.alternativeto.net/software/stirling-pdf/about/), [Stirling docs](https://docs.stirlingpdf.com/Modes%20and%20Licensing)
- **Okular**: form filling feels "hopelessly clunky" (one user). It can verify but historically not *create* signatures (older wiki). By default it stores annotations outside the PDF; writing them into the file is optional. — [AlternativeTo](https://alternativeto.net/software/okular/about), [KDE UserBase](https://userbase.kde.org/Okular)
- **Firefox PDF.js**: added text, draw and signature tools in Fx106 (2022) and highlighting later. Users still ask for digital signatures and strikethrough. Saving opens a "browse" dialog, whereas Edge saves in one click. Extensions cannot extend the built-in viewer. — [Mozilla Connect](https://connect.mozilla.org/t5/ideas/improve-pdf-editing-functionality/idc-p/38518), [Mozilla Connect: features needed](https://connect.mozilla.org/t5/discussions/features-needed-in-firefox-pdf-viewer/m-p/27108), [support.mozilla.org](https://support.mozilla.org/zu/questions/1256512)
- **Edge** is cited by users as having the best annotation/editing of the browser viewers. — [Mozilla Connect](https://connect.mozilla.org/t5/ideas/improve-pdf-editing-functionality/idc-p/18093) [Report]

### Inferences
- The sharpest emotional complaint is **"I paid and the deal changed"** (GoodNotes 5→6, LiquidText, PDF Expert lifetime that doesn't cover mobile or AI). A licence that cannot be revoked (MIT/Apache) is a selling point to state on the landing page. [Opinion]
- Acrobat's "Disable new Acrobat" escape hatch shows that redesigns of a tool people use daily alienate them. lamina should keep its layout stable and let users rearrange it, rather than forcing redesigns. [Opinion]
- Preview's silent auto-save and annotation mangling suggest that **"never modify the original without telling me"** (explicit save, an incremental-save option, an indicator for non-destructive edits) is a trust feature. This maps onto lamina's save planner (ADR 0007).

### Gaps
- I could not read Reddit directly (r/pdf, r/macapps, r/ipad, r/academia). Only one mirrored r/GoodNotes thread surfaced.
- App Store and Play Store review summaries for Acrobat mobile were not retrievable, so complaints about AI Assistant pop-ups or nags are **unverified**.
- I found no 2025–2026 Verge, MacStories, Ars or PCMag reviews in search results.

## 3. Concrete performance failures (startup, large/vector files, memory, battery)

### Takeaway
Documented failures cluster around Acrobat: cold start, hangs, about 2 GB footprint, and iPad rendering of engineering drawings so slow it is "unusable" while the iPad's built-in viewer has no problem. Vector- and resource-heavy files (CAD sheets, maps) are the general stress case, and the usual fix is tiled, page-level rendering. Sumatra is the speed benchmark ("opens almost like a plain text document").

### Cited Findings
- Acrobat's footprint is about 2.04 GB as reported by Windows (Nov 2025). — [Adobe Community](https://community.adobe.com/questions-9/bloated-inefficient-product-cancelling-subscription-1300380) [Report]
- Acrobat is slow to start up and laggy on Windows 11. — [Adobe Community](https://community.adobe.com/t5/acrobat-discussions/acrobat-very-slow-to-start-up-and-very-laggy-when-using-it-on-windows-11/m-p/13977583/highlight/true) [Report]
- On iPad Pro, drawings are "so slow to render in Acrobat Reader ... unusable", with "no such problem with the native iPad file viewer", and OneDrive renders the same files "near instantly". — [Adobe Community: PDFs slow to render on iPad Pro](https://community.adobe.com/t5/acrobat-reader-mobile-discussions/pdfs-slow-to-render-on-ipad-pro/m-p/12435505) [Report, Old/undated]
- Slowness comes from content streams that reference many XObjects and images, and from complex vector content. Re-saving through Acrobat Pro "flattened and optimized" one slow file. — [Nutrient: What contributes to slow PDF rendering](https://www.nutrient.io/blog/what-contributes-to-slow-pdf-rendering/), [Radaee forum](https://www.radaeepdf.com/forum/cordova/3681-slow-performance-for-graphic-heavy-pdf-s)
- One SDK viewer used over 2 GB on a drawing set by holding full-page images in memory. The fix was page-level/visible-only rendering. — [Syncfusion forum](https://www.syncfusion.com/forums/133110/pdfviewer-very-slow-to-display-loaded-document); techniques for image-heavy scrolling — [Syncfusion blog](https://www.syncfusion.com/blogs/post/smooth-scrolling-performance-image-heavy-pdfs)
- Sumatra is "ridiculously fast, remarkably lightweight" ([Opinion], TechRadar), and an HN user says it opens "almost like a plain text document". — [TechRadar](https://www.techradar.com/reviews/sumatra-pdf-reader), [lafabriquedunet](https://www.lafabriquedunet.fr/logiciel/sumatra-pdf)
- Sumatra's own feedback board has a top request titled "keep it very very lightweight". — [Sumatra feedback](https://sumatrapdf.userjot.com/board/p/keep-it-very-very-lightweight)
- Xodo's free version sometimes lags on low-end Windows machines. — [SoftwareSuggest](https://www.softwaresuggest.com/xodo) [Report]
- LiquidText is said to have crash and UI-freeze complaints, but the source is an AI-generated aggregator (medium confidence, **unverified**). — [marlvel.ai](https://marlvel.ai/api/llm/apps/business/liquidtext-prepaid)

### Inferences
- lamina's performance targets should be published and measured: time-to-first-page for cold start, peak memory on a vector-heavy CAD sheet, and scroll frame rate on a 1000-page scanned book. "Faster than Acrobat on the same file" is a demo users can verify themselves. [Opinion]
- Progressive, tiled rendering of the visible region with a cancellable render queue answers the iPad drawing-set complaint directly. That matters for the later mobile app.

### Gaps
- I found no quantitative benchmarks (ms or MB) comparing apps, and no data on mobile battery drain for any PDF app.
- I found no large-file performance reports specific to PDF Expert or GoodNotes.

## 4. UX patterns users single out as good

### Takeaway
Praise falls into four patterns: **speed and minimal UI** (Sumatra, Preview), **reflowed mobile reading** (Acrobat Liquid Mode, which TIME named a best invention of 2023 per readabilitymatters), **spatial research workspaces** (LiquidText), and **handwriting/stylus feel** (GoodNotes). Browser viewers win on zero-install convenience, and Edge's one-click save is noted.

### Cited Findings
- Liquid Mode uses AI to reflow text, tables, bullets and headers for small screens, with adjustable text size and line/character spacing. The original is left unchanged. It is free on iOS, Android and Chromebook. — [Adobe help: Liquid Mode](https://helpx.adobe.com/ca/acrobat/mobile/view-manage-files/liquid-mode.html), [Adobe blog 2020](https://blog.adobe.com/en/publish/2020/12/10/adobe-delivers-enhanced-pdf-reading-experience-for-accessibility) [Old], [TechCrunch 2020](https://techcrunch.com/2020/09/23/adobes-liquid-mode-uses-ai-to-automatically-redesign-pdfs-for-mobile-devices) [Old]
- A low-vision user called Liquid Mode "almost magical". It won TIME Best Inventions recognition (2023, per readabilitymatters). — [readabilitymatters](https://readabilitymatters.org/articles/time-recognizes-adobe-liquid-mode-as-best-invention-of-2023)
- Liquid Mode's limits: no reflow on forms or digital-signature PDFs, and binders convert only partly. — [Adobe help](https://helpx.adobe.com/ca/acrobat/mobile/view-manage-files/liquid-mode.html), [Adobe Community: binder](https://community.adobe.com/t5/acrobat-reader-mobile-discussions/why-doesn-t-liquid-mode-work-for-an-entire-binder/td-p/13167886)
- Adobe Design's write-up on designing mobile-friendly PDFs. — [adobe.design](https://adobe.design/ideas/mobile-friendly-pdfs-for-a-world-on-the-go)
- Sumatra is praised for speed, small install size and being open source. — [AlternativeTo](https://alternativeto.net/software/sumatra-pdf/about), [dedoimedo](https://www.dedoimedo.com/computers/sumatra-pdf.html)
- Okular offers a "Copy Text (Without line breaks)" action, a small feature users value. Its sidebar offers thumbnails, reviews and bookmarks. — [KDE invent MR 1157](https://invent.kde.org/graphics/okular/-/merge_requests/1157), [apps.kde.org](https://apps.kde.org/et/okular)
- In Preview, holding Option while dragging draws a rectangular text selection, which works around multi-column text. — [ruk.ca](https://ruk.ca/comment/20377)
- Edge's annotation tools are rated "best among browsers" and save in one click. — [Mozilla Connect](https://connect.mozilla.org/t5/ideas/improve-pdf-editing-functionality/idc-p/18093) [Report]
- Firefox recognises PDF tags well for screen readers, better than Chrome or Edge (2024 anecdote). — [WebAIM list](https://webaim.org/discussion/mail_message?id=50258)
- Capterra users praise Foxit Reader as a "great lightweight PDF viewer". — [Capterra](https://www.capterra.com/p/175376/Foxit-PDF-Reader/reviews/)

### Inferences
- A layered UI fits the evidence: open instantly in a minimal Sumatra- or Preview-like view, and reveal tools only on intent. Reflow ("reading mode") is a feature that sets apps apart on mobile and for accessibility. lamina could do it **without cloud AI**, using structure tags plus layout analysis in the Rust core. That would be privacy-preserving where Adobe's is cloud and AI. [Opinion]

### Gaps
- I found no sourced 2024–2026 user praise of PDF Expert's reading mode or reflow, LiquidText's workspace, or GoodNotes's stylus latency. Those patterns are well known from my training data but are **unsourced here**.
- I found no data on iOS Files or Quick Look markup.

## 5. Specific friction points

### Takeaway
Friction recurs in seven areas:
- multi-column selection and copy that interleaves columns
- hard line breaks in pasted text
- flat (non-fillable) forms that need manual text boxes
- confusion between e-signatures and certificate digital signatures, including fields recipients can't see
- "black box" redactions that leave text recoverable
- forms that render differently across apps
- merge and compress rationed behind web-tool caps

### Cited Findings
- **Columns**: drag-select across columns yields "first line of column 1, first line of column 2, …". Removing line breaks cannot recover reading order. — [proz.com forum](https://deu.proz.com/forum/office_applications/368735-pdf_selection_mode_for_column.html), [dev.to textbases](https://dev.to/textbases/how-to-remove-line-breaks-from-copied-pdf-text-without-destroying-paragraphs-3dh1); long-standing bugs in Evince — [GNOME evince #44](https://gitlab.gnome.org/GNOME/evince/issues/44), [Launchpad 33288](https://bugs.launchpad.net/bugs/33288) [Old]; Firefox — [Bugzilla 1136674](https://bugzilla.mozilla.org/show_bug.cgi?id=1136674) [Old]
- **Line breaks**: the clipboard keeps visual line endings as newlines, while some files lose them entirely so lines run together. — [Skim manual](https://skim-app.sourceforge.io/manual/SkimHelp_8.html), [Adobe Community](https://community.adobe.com/questions-671/how-to-remove-extra-line-breaks-when-copy-pasting-after-exporting-to-pdf-865214)
- **Flat forms**: they have no fields, so clicking gives no cursor and users must place free text boxes by hand. Acrobat highlights real fields in blue in Fill & Sign. — [pdf.net](https://pdf.net/blog/how-to-fill-out-and-sign-pdf-forms), [Smallpdf blog](https://compress.smallpdf.com/blog/fill-and-sign-pdf-forms)
- **Cross-app form rendering**: forms filled in Preview display wrongly in Acrobat. One thread advises not to mix PDF viewers. — [Apple Discussions 254193728](https://discussions.apple.com/thread/254193728), [Apple Discussions 3255797](https://discussions.apple.com/thread/3255797) [Old]
- **Signatures**: Adobe Community threads include "So confused: e-signature vs digital signature" and "I'm confused about Acrobat's different kinds of signature choices". Recipients get prompted to set up a digital ID when they meet a certificate field. Digital-signature fields don't show as fillable to recipients. A university IT guide exists just to explain the difference. — [Adobe Community: so confused](https://community.adobe.com/questions-9/so-confused-e-signature-vs-digital-signature-1297537), [Adobe Community: different kinds](https://community.adobe.com/t5/acrobat-discussions/i-m-confused-about-acrobat-s-different-kinds-of-signature-choices/td-p/15080667), [Adobe Community: not fillable](https://community.adobe.com/t5/acrobat-discussions/digital-signature-not-showing-up-as-fillable-field-when-recipient-receives-document/td-p/15608331), [Southern CT State Univ. guide (PDF)](https://inside.southernct.edu/sites/default/files/inline-files/E-Signature%20Fields%20vs%20Digital%20Signature%20Fields%20-%20Users%20Being%20Prompted%20to%20Configure%20Digital%20Certificates%20Signatures%20or%20IDs_0.pdf)
- **Redaction**: black boxes are shapes, and the text underneath remains. OCR text layers in scans survive, and metadata leaks. Eugene Volokh (Sep 2025) saw a failed redaction "in a recent filing from an AmLaw top-20 law firm". A study suggests tens of thousands of PACER documents may have failed redactions. — [Volokh Conspiracy, Sep 2025](https://reason.com/volokh/2025/09/23/avoid-super-embarrassing-redaction-failures-3/), [ABA Journal](https://www.abajournal.com/news/article/tens_of_thousands_of_pacer_documents_could_have_failed_redactions_study_sug) [Old], [dev.to: "The black box is a shape, not a delete key"](https://dev.to/haruodev/the-black-box-in-your-pdf-is-a-shape-not-a-delete-key-1nc3), [Syncfusion](https://www.syncfusion.com/resource-center/why-black-boxes-dont-protect-sensitive-data/); an HN thread on a repeat failure — [hn.nuxt.dev 46368946](https://hn.nuxt.dev/item/46368946) (details not verified)
- **Merge/compress paywalls**: Smallpdf allows 2 free tasks/day, iLovePDF caps per hour, and Xodo Web allows 1 download/day. A wave of "free, no limits" alternatives markets against this. — [codaone](https://www.codaone.ai/compare/ilovepdf-vs-smallpdf), [Xodo](https://xodo.com/blog/xodo-plans-explained), [wildandfreetools](https://wildandfreetools.com/blog/ilovepdf-smallpdf-free-alternative-compress-merge-split/), [mioffice (archived)](https://web.archive.org/web/20260716200235/https://mioffice.ai/blog/ilovepdf-alternative-free-no-limits)
- **Firefox save flow**: users find the save-as dialog after editing awkward compared with Edge. — [Mozilla Connect](https://connect.mozilla.org/t5/ideas/improve-pdf-editing-functionality/idc-p/38518)

### Inferences (design implications for lamina) [Opinion]
- **Selection**: use reading-order-aware selection based on structure tags or layout analysis, with column-constrained drag. Offer "Copy as paragraph" (joins hyphenated and soft-wrapped lines) as the default and "Copy exact" as the alternative.
- **Flat forms**: detect fields automatically (boxes, underlines, "Name: ____") and offer one-tap "make fillable" locally. Snap free text to the baseline.
- **Signatures**: use two clearly named actions, "Add signature image (not legally verifying)" and "Sign with certificate (tamper-evident)". Show a plain-language panel on what each proves, and show a validation badge on open.
- **Redaction**: make it a true removal workflow, with mark → review → apply that rewrites the content streams. Also remove the OCR layer and scrub metadata. Then run a **post-apply self-check** that searches the output for the redacted strings and refuses to save if they are found. A highlight drawn as a black box should never be confusable with a redaction. lamina's object-level access in the Rust core makes the verification step possible.
- **Merge, compress, split**: free and unlimited, run locally in WASM, with "no upload" stated visibly in the UI.

### Gaps
- I found no quantitative user-research data (for example, % of users affected) for any of these friction points.

## 6. Gaps a privacy-first, fast, free open-source app could fill

### Takeaway
The open spaces are:
- a **cross-platform** fast reader. Sumatra is Windows-only, Preview is Mac-only and Okular is Linux-first.
- **local-only web tools** that replace upload-and-ration services
- **trustworthy** saving, redaction and signatures
- **reflow and accessibility without cloud AI**
- a **licence that cannot be revoked**, against subscription churn

Competitors on these fronts include Stirling-PDF (self-hosted server, utilitarian UI, paid enterprise tier) and PDF.js (browser-embedded and limited).

### Cited Findings
- Users on Mac and Linux miss Sumatra, and people are looking for open-source tools because of Adobe's revenue push. — [GeekNews / HN summary](https://news.hada.io/topic?id=11501)
- Web tools upload every file to their servers, with retention of 1–2 h. — [codaone](https://www.codaone.ai/compare/ilovepdf-vs-smallpdf), [pdnob](https://www.pdnob.com/pdf-tips/is-smallpdf-safe.html)
- Stirling-PDF needs a server for OCR, conversion, compression and repair, its UI is "functional rather than polished", and enterprise features are paid. — [Stirling docs](https://docs.stirlingpdf.com/Modes%20and%20Licensing), [aicybr](https://aicybr.com/blog/stirling-pdf-self-hosting-guide)
- The PDF.js viewer can't be extended by add-ons and lacks digital signatures. — [support.mozilla.org](https://support.mozilla.org/zu/questions/1256512), [Mozilla Connect](https://connect.mozilla.org/t5/discussions/features-needed-in-firefox-pdf-viewer/m-p/27108)
- GoodNotes's one-time licence is limited to the Apple ecosystem. — [brainbound](https://www.brainbound.blog/goodnotes-membership-cost-features)
- Okular stores annotations outside the PDF by default, which hurts portability. Preview damages other apps' annotations. — [KDE UserBase](https://userbase.kde.org/Okular), [Eclectic Light](https://eclecticlight.co/2025/08/14/inside-pdf-annotations/)
- Foxit's 2025 security advisories and the 18 vulnerabilities Talos disclosed in 2018 ([Old]) show the attack surface of C/C++ readers. — [Cisco Talos](https://blogs.cisco.com/security/talos/vulnerability-spotlight-multiple-issues-in-foxit-pdf-reader-2), [Cyber Centre AV25-510](https://cyber.gc.ca/en/alerts-advisories/foxit-security-advisory-av25-510)

### Inferences [Opinion]
- **Positioning**: "Opens instantly, works offline, never uploads, never asks you to sign in, and everything is free forever." Each clause answers a sourced grievance: Acrobat cold start and bloat, web-tool uploads, Acrobat mobile sign-in, and task caps plus subscriptions.
- **Security as a feature**: a memory-safe Rust parser, with `Limits` per ADR 0003, is a credible answer to the steady stream of reader CVEs (Foxit/Talos). It deserves a line in marketing.
- **Interoperability as trust**: write annotations and form values *into* the PDF per spec (unlike Okular's sidecar default). Never silently auto-save (unlike Preview). Generate appearance streams so other viewers see filled values (Preview→Acrobat breakage).
- **Avoid the Stirling trap**: Stirling feels like a server toolbox. lamina's advantage is a *document-centric* reader and editor UI where tools act on the open document. Stirling's model is a list of tools, each run on uploaded files.
- **Do not imitate**: AI bundled behind tiers, forced redesigns, nagging dialogs, or cloud upsell at save time.

### Gaps
- I found no market-size or usage-share data for these segments.
- Whether users would accept the lack of cloud sync or collaboration that a strictly local app implies is not evidenced in sources found here.
- Reddit, App Store and Play Store review mining, and 2025–2026 tech-press reviews, were not reachable in this session. Those sources would strengthen sections 2–4.
