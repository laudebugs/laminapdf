# Win on trust, speed and restraint: product and UX research for lamina's apps

- **Date:** 2026-10-29
- **Notes:** [competitive teardown](notes/competitive-teardown.md) · [personas and workflows](notes/personas-workflows.md) · [UI/UX patterns](notes/ui-ux-patterns.md) · [engines and SDKs](notes/engines-sdks.md) · [differentiation and MVP](notes/differentiation-mvp.md)
- **Plan that turns this into work:** [UI/UX plan](../../plans/ui-ux.md)

**Bottom line.** The gap is not missing features. Incumbents have taught users to distrust PDF software. Acrobat ships about a 2 GB install with a redesign so disliked that Adobe keeps a "Disable new Acrobat" toggle. GoodNotes, LiquidText and PDF Expert changed the deal on people who had already paid. Preview silently rewrites files and damages other apps' annotations. Web tools ration tasks and upload every file. Legal teams keep leaking text under black boxes, as recently as the DOJ Epstein releases of December 2025.

lamina's planned architecture already answers most of these grievances. Its pieces are a memory-safe Rust core with `Limits`, an op log with undo, a save planner that can append without touching signed bytes, and a zero-upload web reader. The product job is to make those properties **visible and verifiable** in the UI, rather than to win on tool count. Stirling-PDF and dozens of "no upload" WASM sites already compete on tool count.

The technical case for PDFium-WASM as the default renderer, behind a tile-based `RenderBackend` with hayro as the pure-Rust alternative, holds up in 2026. Every other free engine is either AGPL/GPL or immature. The commercial SDKs (a median of about $25k a year for Apryse, from €5k for Nutrient) are benchmarks to beat, not dependencies. None of the open-source engines documents a signature-preserving incremental save, so lamina's save planner is a real moat.

**Opinion:** the UX should be a calm, docked "inverted-L" layout that opens instantly into a Sumatra-like reading view, reveals tools through modes and a command palette, and never floats palettes over the page. On mobile it should defer to the OS selection menu and keep actions in a thumb-reachable bottom bar. Ship a reader first (v0.1), then trustworthy annotation and form saving, then organising and verified redaction, then local OCR, search and reflow. Native mobile should wait for a spike comparing Tauri mobile with UniFFI and native UI.

> **How to read this report.** Claims are tagged **[Fact]** (vendor-documented, or reported by more than one independent source), **[Report]** (individual user anecdote), **[Estimate]** (a number inferred or modelled here, not measured) and **[Opinion]** (a recommendation or judgement by the research team or this report). Price figures come mostly from aggregators that disagree with each other; verify them before quoting. Several primary sites were unreachable from the research session. They are listed in [the evidence gaps section](#several-primary-sources-were-unreachable-and-the-gaps-matter) near the end.

## Competitors fail on bloat, broken deals and silent damage, not on missing features

### Pricing has split the market four ways, and AI is the new upsell lever

**[Fact]** The 2026 market has four business models:

- **Adobe: subscription-only, with AI in a higher tier.** Acrobat Studio launched in Q3 2025 at **$24.99/month**, bundling Acrobat Pro, AI Assistant, PDF Spaces and Adobe Express Premium ([PricingSaaS](https://pricingsaas.com/companies/adobe.creativecloud/diffs/2025Q3); [Schneider licensing guide](https://www.schneider.im/adobe-acrobat-studio-licensing-guide-plans-features-pricing-comparison/)). Pro sits around $19.99/month and Standard around $14.99/month ([thewearify](https://thewearify.com/acrobat-professional-price/)). The AI Assistant add-on price is unresolved: sources say $1.99 or $4.99.
- **Challengers that sell "pay once".**
  - Foxit sells PDF Editor 14 perpetually for about **$209.99** ([casrai](https://web.archive.org/web/20260926000124/https://casrai.org/guides/foxit-pdf-editor-price)).
  - Xodo PDF Studio is **$240 perpetual** ([Xodo plans explained](https://xodo.com/blog/xodo-plans-explained)).
  - PDF Expert's Mac lifetime licence was repeatedly discounted to $69.97–$79.99 through 2026 ([Macworld](https://macworld.com/article/3217258/pay-once-skip-adobes-subscription-pdf-expert-for-mac-is-79-99-42-off-for-life.html)). That licence excludes AI and translation and does not carry over to iPhone or iPad ([Frontdesk Review](https://frontdeskreview.com/software/pdf-tools/pdf-expert.md)).
- **Freemium web tools that ration tasks, not features.**
  - Smallpdf allows about two free tasks a day.
  - iLovePDF's free limits are reported inconsistently ([codaone](https://www.codaone.ai/compare/ilovepdf-vs-smallpdf)).
  - Xodo Web allows **one file download a day** ([Xodo](https://xodo.com/blog/xodo-plans-explained)).
- **Free or open-source viewers.** These include Sumatra, Okular, the browser viewers and Preview. Stirling-PDF is open-core and already charges for SSO, SAML and audit logs ([Stirling docs](https://docs.stirlingpdf.com/Paid-Offerings/)).

The pattern that matters is that AI is now the reason to move a user up a tier: Adobe Studio, PDF Expert's separate AI subscription, and Foxit's AI credits. Foxit's allowance reportedly rose from 20 to 300 credits a month in August 2026, which Foxit has not confirmed ([PricingSaaS](https://pricingsaas.com/companies/foxit.pdfeditor/diffs/2026W37)). Users react to the price, not to the capability. An Adobe forum thread said the Studio rollout "critically damaged Acrobat's credibility amongst daily professional users" ([Adobe Community](https://community.adobe.com/t5/acrobat-discussions/meet-the-new-adobe-acrobat-studio/td-p/15465617), **[Report]**).

### The friction audit by app

The documented complaints cluster by app. All rows are **[Report]** unless marked otherwise.

| App | What users praise | What users complain about | Lesson for lamina **[Opinion]** |
|---|---|---|---|
| **Acrobat** | Liquid Mode reflow on mobile (TIME Best Invention 2023, per [readabilitymatters](https://readabilitymatters.org/articles/time-recognizes-adobe-liquid-mode-as-best-invention-of-2023)) | About a **2.04 GB** footprint ([Adobe Community](https://community.adobe.com/questions-9/bloated-inefficient-product-cancelling-subscription-1300380)). Slow on an i7 with 16 GB RAM ([thread](https://community.adobe.com/questions-9/poor-performance-on-a-capable-system-i7-16gb-ram-ssd-1305162)). New UI "horrible", with a revert toggle kept ([thread](https://community.adobe.com/questions-9/why-does-acrobat-have-a-new-interface-and-how-do-i-switch-back-to-the-classic-interface-1297830)). iPad drawings "unusable" while the native viewer copes ([thread](https://community.adobe.com/t5/acrobat-reader-mobile-discussions/pdfs-slow-to-render-on-ipad-pro/m-p/12435505)). **[Fact]** Sign-in is required for free online features ([Adobe FAQ](https://helpx.adobe.com/acrobat/mobile-app-faq.html)) | Publish speed targets. Keep the layout stable. Use tiled rendering. No account wall |
| **Foxit** | "Lightweight", "cleaner" Reader ([Capterra](https://www.capterra.com/p/175376/Foxit-PDF-Reader/reviews/)) | **[Fact]** Several 2025 security advisories and 18 vulnerabilities disclosed by Cisco Talos ([Talos](https://news-blogs.cisco.com/security/talos/vulnerability-spotlight-multiple-issues-in-foxit-pdf-reader-2); [Cyber Centre AV25-623](https://cyber.gc.ca/en/alerts-advisories/foxit-security-advisory-av25-623)) | A memory-safe parser is a marketable security story |
| **PDF Expert** | Mac polish; Night and Sepia reading themes ([help](https://pdfexpert.helpjuice.com/turn-on-the-dark-mode)) | The lifetime licence doesn't cover mobile or AI | Make the licence that cannot be revoked a headline |
| **GoodNotes 6** | Handwriting feel (no source found in this pass) | Weak upgrade path from v5, subscription nags, AI pushed hard ([Malay Mail](https://www.malaymail.com/news/tech-gadgets/2023/08/24/is-it-all-good-with-the-updated-goodnotes-6/87000), [Feedbagel](https://feedbagel.com/post/goodnotes-6-transition-to-subscription-model-frustrates-users-who-bought-lifetim)) | Never change the deal |
| **LiquidText** | Spatial research workspace (no source found in this pass) | Subscription "restrictive and costly, especially for students"; sync costs extra ([GetApp](https://www.getapp.com/collaboration-software/a/liquid-text/)) | Spatial workspace is an academic opportunity, but not for the MVP |
| **macOS Preview** | Simplicity; Option-drag rectangular selection across columns ([ruk.ca](https://ruk.ca/comment/20377)) | Auto-saves on close and "mutilates" other editors' annotations ([Eclectic Light, Aug 2025](https://eclecticlight.co/2025/08/14/inside-pdf-annotations/)). "EATs" JavaScript forms ([Michael Tsai](https://mjtsai.com/blog/?p=49000)). Radio-button values lost on reopen ([Apple forums](https://developer.apple.com/forums/thread/735073)). No certificate signatures | Never modify the file silently. Write to spec, with appearance streams |
| **SumatraPDF** | "Ridiculously fast", opens "almost like a plain text document" ([TechRadar](https://www.techradar.com/reviews/sumatra-pdf-reader)) | Windows-only, limited editing. Mac and Linux users miss it ([HN summary](https://news.hada.io/topic?id=11501)) | The speed benchmark, and the cross-platform gap to fill |
| **Okular** | "Copy text without line breaks" ([KDE MR 1157](https://invent.kde.org/graphics/okular/-/merge_requests/1157)) | Clunky forms. **[Fact]** Annotations stored outside the PDF by default ([KDE UserBase](https://userbase.kde.org/Okular)) | Write annotations into the file |
| **Firefox PDF.js / Edge** | Zero install. Firefox handles tags well for screen readers ([WebAIM list](https://webaim.org/discussion/mail_message?id=50258)). Edge saves in one click ([Mozilla Connect](https://connect.mozilla.org/t5/ideas/improve-pdf-editing-functionality/idc-p/18093)) | No digital signatures; cannot be extended; awkward save-as dialog in Firefox ([Mozilla Connect](https://connect.mozilla.org/t5/ideas/improve-pdf-editing-functionality/idc-p/38518)) | One-click save to the original where the platform allows it |
| **Smallpdf / iLovePDF / Xodo Web** | Task-oriented convenience | Daily task caps; **[Fact]** every file is uploaded, with 1–2 h stated retention ([pdnob](https://www.pdnob.com/pdf-tips/is-smallpdf-safe.html)) | Unlimited local operations, with "no upload" visible in the UI |
| **Stirling-PDF** | About 95k GitHub stars; local "edit, sign, redact" ([repo](https://github.com/Stirling-Tools/Stirling-PDF)) | Interface "functional rather than polished" ([aicybr](https://aicybr.com/blog/stirling-pdf-self-hosting-guide)). **[Fact]** Desktop mode without a server lacks OCR, conversion, compression and repair ([docs](https://docs.stirlingpdf.com/Modes%20and%20Licensing)) | Be centred on the open document, not a list of tools |

### The friction that recurs everywhere

Five frictions recur in every app, and each has a documented mechanism:

- **Multi-column copy.** Copying across columns interleaves them, line 1 of column 1 then line 1 of column 2, and removing line breaks afterwards cannot restore reading order ([proz.com](https://deu.proz.com/forum/office_applications/368735-pdf_selection_mode_for_column.html); [GNOME evince #44](https://gitlab.gnome.org/GNOME/evince/issues/44)).
- **Flat forms.** Forms with no fields give no cursor, so users place text boxes by hand ([pdf.net](https://pdf.net/blog/how-to-fill-out-and-sign-pdf-forms)).
- **Forms that break across apps.** A form filled in Preview displays wrongly in Acrobat ([Apple Discussions](https://discussions.apple.com/thread/254193728)).
- **Two kinds of signature.** Adobe's own forum has threads titled "So confused: e-signature vs digital signature" ([Adobe Community](https://community.adobe.com/questions-9/so-confused-e-signature-vs-digital-signature-1297537)), and a university IT office published a guide just to explain the difference ([SCSU guide](https://inside.southernct.edu/sites/default/files/inline-files/E-Signature%20Fields%20vs%20Digital%20Signature%20Fields%20-%20Users%20Being%20Prompted%20to%20Configure%20Digital%20Certificates%20Signatures%20or%20IDs_0.pdf)).
- **Fake redaction.** Black boxes are drawn shapes, and the text underneath stays in the file ([dev.to](https://dev.to/haruodev/the-black-box-in-your-pdf-is-a-shape-not-a-delete-key-1nc3)). Eugene Volokh saw it in a filing from an AmLaw top-20 firm in September 2025 ([Volokh Conspiracy](https://reason.com/volokh/2025/09/23/avoid-super-embarrassing-redaction-failures-3/)).

The emotional core of the complaints is **"I paid and the deal changed"** and **"it changed my file without asking"**. An app under MIT OR Apache-2.0 that saves explicitly, and incrementally by default, answers both directly **[Opinion]**.

## Three personas, and two deal-breakers that decide business adoption

### The 70 / 20 / 8 / 2 task split is unverified, and probably wrong for the web

No public dataset splits PDF activity by task, not from Adobe, Smallpdf or iLovePDF, and not from HCI log studies. The hypothesis of 70% viewing, 20% annotating, 8% organising and 2% converting can be neither confirmed nor refuted with data.

The indirect evidence pulls two ways:

- **Reading is large.** Adobe reports over **one billion files read in Liquid Mode** by February 2024 ([Adobe blog](https://blog.adobe.com/en/publish/2024/02/08/adobe-liquid-mode-recognized-edison-awards-finalist)).
- **The web-tool economy runs on tasks.** iLovePDF reports **860M+ unique visitors a year and about 20M a day**. It started with only merge and split, and added tools "because users kept asking for the next step in the workflow" ([iLovePDF blog](https://www.ilovepdf.com/blog/ilovepdf-15-years-anniversary-milestones)). Smallpdf claims over a billion users, with no per-tool split ([Smallpdf](https://smallpdf.com/de/about)).

**[Estimate]** Viewing and search probably dominate *time in app* in a reader, at 60–75% of sessions. Organising and converting could make up **20–40% of first-visit intents** on a web app. Treat 70/20/8/2 as a time-in-app prior only, and validate both splits with opt-in, content-free analytics. The iLovePDF origin story also argues for chained workflows (merge → compress → sign → share) over isolated tools **[Opinion]**.

### What each persona needs

The three personas differ less in which features they want than in what makes them leave.

**Academics and researchers.** Their tools must round-trip with their citation manager:

- **[Fact]** Readwise imports highlights only when they are standard PDF annotations. Highlights from MarginNote, GoodNotes or Notability cannot be extracted ([Readwise docs](https://docs.readwise.io/readwise/docs/importing-highlights/pdf)).
- **[Fact]** Zotero 7 (August 2024) is the benchmark reader: split view, dark mode for page content, and EPUB and snapshot annotation ([Zotero blog](https://www.zotero.org/blog/zotero-7/)). But it stores annotations in its own database rather than in the PDF ([Uppsala UB](https://libguides-en.ub.uu.se/zotero/PDFReader)).
- Research prototypes show the next step. Semantic Reader's "Paper Cards" pop up when you click an inline citation, "reducing the interaction cost of jumping back and forth" to the reference list ([CACM](https://cacm.acm.org/research/the-semantic-reader-project)). ScholarPhi's in-place definitions helped researchers at every experience level, though the formative study was small (n=9) ([arXiv](https://arxiv.org/abs/2009.14237v2)).
- Device data is old. A 2015 survey (n=416) found that most researchers still printed for close reading ([Moed et al.](https://cris.biu.ac.il/en/publications/accessing-reading-and-interacting-with-scientific-literature-as-a/)).

**[Opinion]** lamina should be the reader that writes **interoperable annotations into the file**, and should integrate with Zotero rather than compete with it.

**Business and legal professionals.** They have two hard deal-breakers.

- **Recoverable redaction.** Manafort's 2019 filing used black boxes over live text, and reporters copied the hidden passages ([ABA Journal](https://www.abajournal.com/news/article/paul-manaforts-attorneys-failed-at-redacting-learn-how-to-do-it-right)). The December 2025 DOJ Epstein releases repeated the failure, confirmed for at least one document ([Mediaite](https://www.mediaite.com/media/news/doj-redactions-of-some-epstein-files-easily-revealed-with-copy-and-paste); [GV Wire](https://gvwire.com/2025/12/24/redacted-material-in-some-epstein-files-is-easily-recovered/)). A study suggests tens of thousands of PACER documents may have failed redactions ([ABA Journal](https://www.abajournal.com/news/article/tens_of_thousands_of_pacer_documents_could_have_failed_redactions_study_sug)).
- **Signatures without legal standing.** Under eIDAS Article 25, only a *qualified* electronic signature has the effect of a handwritten one. A qualified signature needs a qualified certificate and a qualified signature creation device (QSCD). PAdES is the PDF *format* for these signatures, and its legal level depends on the certificate and device ([Wikipedia: QES](https://en.wikipedia.org/wiki/Qualified_electronic_signature); [Adobe Sign: EU legality](https://helpx.adobe.com/ca/sign/using/legality-european-union.html)). These are secondary sources; check against the Official Journal text and ETSI EN 319 142.
- **Email size limits** set a practical compression target. Gmail caps outgoing messages at **25 MB**, and Base64 encoding inflates a 20 MB attachment to about 27 MB ([IONOS](https://www.ionos.com/digitalguide/e-mail/technical-matters/gmail-attachment-size-limit.md)). "Compress to fit email" is a real job, not a vanity feature.

**Casual mobile users.** They need four things: fast open from the share sheet, reflowed reading, camera scan, and a finger signature. Their deal-breakers are sign-up walls and uploads. Liquid Mode proves the reflow demand, and its limits mark the opening for lamina:

- it processes in Adobe's cloud, which drew privacy concerns ([The Register](https://www.theregister.com/2020/09/24/adobe_liquid_mode_pdf_mobile/))
- it skips files over 200 pages, forms and signed PDFs ([Adobe Help](https://helpx.adobe.com/ca/acrobat/mobile/view-manage-files/liquid-mode.html))

**Accessibility is not a fourth persona but a property of all three.** In WebAIM's 2019 survey, **75.1%** of screen-reader users said PDFs are likely to pose significant accessibility problems, against 30.9% for Word documents ([WebAIM](https://webaim.org/blog/screen-reader-survey8-results/); 2019 data, so older). Testing by the Information Access Group found that the viewer matters as much as the tagging ([Information Access Group](https://informationaccessgroup.com/comparing-how-well-pdf-viewer-applications-work-with-screen-readers/)).

## Docked chrome, modes and a palette beat floating toolbars

### Desktop and web: an inverted-L layout with contextual disclosure

The strongest UX evidence is negative, and it is recent:

- **[Fact]** Figma's UI3 launched with floating panels and then **reattached the navigation and properties panels** after user feedback. Panels float only in "Minimize UI" mode, where selecting something on the canvas temporarily brings back the properties panel ([Figma blog](https://www.figma.com/blog/figma-2024-we-shipped-it-you-shaped-it/); Figma Help (offline, no archived copy, 2026-10-30)).
- **[Report]** Acrobat's 2023 floating quick-tools widget drew complaints for covering page content, and Adobe had to make it movable ([Adobe Community](https://community.adobe.com/t5/acrobat-discussions/how-do-i-move-remove-the-quick-actions-toolbar-from-acrobat-pro-2022-003-xxxx/m-p/13946883/highlight/true)).
- **[Fact]** Linear's 2024 redesign centred on the "inverted L": a sidebar plus a top header that frame the main view, with the aim of less visual noise ([Linear](https://linear.app/blog/how-we-redesigned-the-linear-ui)).
- The Office ribbon research adds a quantitative warning, via secondary accounts. About 80% of saves came from the toolbar icon and only 20% from Ctrl+S, so **visible affordances beat shortcuts** for most users ([GNOME blog](https://blogs.gnome.org/bolsh/?p=582)). The ribbon's lasting contribution was contextual tabs that appear only when relevant ([Thurrott](https://www.thurrott.com/office/161324/vaults-office-ribbon)).
- Microsoft's own guidance warns that controls appearing and disappearing unexpectedly make an interface feel unstable ([Microsoft Learn](https://learn.microsoft.com/sk-sk/windows/win32/uxguide/ctrl-progressive-disclosure-controls)).

**[Opinion]** The synthesis for `@laminapdf/ui` has five parts:

- **Top bar.** A slim bar with file actions, a mode switcher, find, and the Cmd/Ctrl+K entry point.
- **Left rail.** A collapsible rail for thumbnails, outline and bookmarks.
- **Canvas.** It fills the rest of the window.
- **Right panel.** A docked panel that opens only when an annotation, field or object is selected.
- **Modes.** Read, Comment, Fill & Sign, Organise and Edit. Each mode swaps in a single secondary toolbar, which is the lightweight version of contextual tabs.

A focus toggle hides both rails, in the style of Figma's Minimize UI. The default state is Read mode, which should look as spare as Sumatra. Tools appear only when the user expresses an intent.

### Selection menus: float them on desktop, never over the OS menu on mobile

**[Fact]** On touch devices the platform owns the selection menu:

- Android uses a floating ActionMode ([Android docs](https://developer.android.com/reference/android/view/ActionMode.html)).
- iOS uses UIEditMenuInteraction.
- On the web, the browser shows its own menu.

Wikimedia's mobile editor found that the native menu covered its own toolbar. On iOS, scrolling hides the copy/paste menu and there is no way to bring it back from code ([Phabricator T202723](https://phabricator.wikimedia.org/T202723)).

**[Opinion]** On desktop, a mini toolbar above the first line of the selection is fine. It should flip below on collision and appear only on `pointerup`, never during a drag. It should be a `role="toolbar"` reachable by keyboard (Shift+F10), and every action in it must also exist in the menu bar and the palette. The floating menu must never be the only path to an action.

On mobile, show a contextual **bottom action bar** while a selection exists (Highlight, Underline, Comment, Copy, More), and leave the native menu alone.

The viewer should only *emit* selection events with client rects, and `@laminapdf/ui` should own the menu. That keeps `@laminapdf/viewer` headless.

### A command palette built on one command registry

**[Fact]** cmdk is the default React palette library:

- MIT licence; requires React 18+ (it uses `useId` and `useSyncExternalStore`); unstyled.
- Supports a custom filter and ranking, async items, and `shouldFilter={false}` for bringing your own ranking.
- The dialog variant uses Radix Dialog, and labelling and ARIA were tested with VoiceOver.
- It has no built-in nested pages. The README shows them through a page stack held in component state ([cmdk README](https://github.com/pacocoursey/cmdk)).

kbar, the alternative, last saw a push in March 2024 ([GitHub: timc1/kbar](https://github.com/timc1/kbar)).

**[Opinion]** The palette is a *view* of a single command registry, not a feature in its own right. That registry should also feed:

- the Tauri native menus (D4b)
- toolbars and context menus
- the shortcut help dialog

Each command carries an id, title, keywords, shortcut, an `enabled(ctx)` check and optional typed arguments. Typing `extract 4-8` should parse the arguments directly. Every row shows its shortcut chip, which is the main way shortcuts become discoverable. The palette follows the APG combobox pattern, with focus kept in the input and `aria-activedescendant` pointing at the active row.

### Mobile ergonomics: bottom bar, sheets with detents, and the viewer owns gestures

**[Fact]** Hoober observed 1,333 people and reported about 49% one-handed use, 36% cradled and 15% two-handed. These figures come via secondary summaries; the original was not fetched ([UXmatters](https://uxmatters.com/columns/mobile-matters)). His 2022 book argues that the popular "thumb zone" folklore is often wrong and that accuracy is best at the centre ([Smashing](https://www.smashingmagazine.com/printed-books/touch-design-for-mobile-interfaces/)).

Other platform facts that shape the layout:

- **[Fact]** Apple sheets support medium and large detents ([Apple HIG](https://developer.apple.com/design/human-interface-guidelines/sheets)).
- **[Fact]** On iOS, `env(safe-area-inset-*)` is zero unless the viewport sets `viewport-fit=cover` ([WebKit bug 272779](https://bugs.webkit.org/show_bug.cgi?id=272779)).
- **[Fact]** WCAG 2.2 AA requires 24×24 CSS px targets (SC 2.5.8) and a single-pointer alternative to every drag (SC 2.5.7) ([TetraLogical](https://tetralogical.com/blog/2023/10/05/whats-new-wcag-2.2)).

**[Opinion]** The mobile layout:

- **Top bar** with title, back and overflow; it auto-hides on scroll.
- **Bottom bar** with Search, Annotate, Pages, Reader and More.
- **Bottom sheets** with medium and large detents for thumbnails, outline and properties.
- **Targets** of at least 44 px.
- **Canvas gestures.** The viewer sets `touch-action: none` and implements pan, pinch and double-tap itself, so browser zoom doesn't fight it. Long-press selects text. Drawing needs an explicit mode or a stylus `pointerType`, so a one-finger pan never draws by accident.

### Reflow, organiser, dark mode and perceived speed

**Reflow quality is bounded by reading order.**

- **[Fact]** Acrobat desktop Reflow cannot fix a bad reading order, and Fondazione LIA's 2024 test found its order did not always match the reading order ([Fondazione LIA](https://www.fondazionelia.org/en/research-and-development/reflowing-pdfs-with-acrobat-reader-we-have-tested-the-new-feature/)).
- **[Fact]** The PDF Association's "Deriving HTML from PDF" (v1.0, 2019, with a second edition in development) defines a tag-tree-to-HTML algorithm for tagged ISO 32000-2 files ([PDF Association](https://www.pdfa.org/resource/deriving-html-from-pdf)).
- **[Fact]** Dual Lab's analysis of about 15M Common Crawl PDFs found that the **tagged share passed 50% in mid-2025** ([Common Crawl group](https://groups.google.com/g/common-crawl/c/_ttmwLV7wuk)). Fewer than 1% claim PDF/UA conformance, so tag quality varies ([PDF Association](https://pdfa.org/wp-content/uploads/2025/10/0-3-11_30-BorisDoubrov-Tagged_PDF_in_the_Wild.pdf)).

**[Opinion]** Build reader mode in two tiers:

1. When the file is tagged, derive the layout from the structure tree. This builds on B8a; read ISO 32000-2 §14.8 first.
2. Otherwise, run a heuristic layout analysis in pure Rust and label the result **"Approximate"**.

Never reflow tables, figures or maths; show them as cropped page images with `/Alt` text. Always offer "back to page" at the matching position. The core emits semantic blocks and `@laminapdf/ui` renders them, which keeps the core bytes-in/bytes-out.

**Page organiser.** **[Fact]** dnd-kit's current version announces drag events through a live region and supports keyboard dragging ([dnd kit](https://dndkit.com/guides/accessibility)). `@dnd-kit/accessibility` is MIT; the licences of `@dnd-kit/dom` and `@dnd-kit/react` were not confirmed. **[Opinion]** Use a multi-select grid with an insertion bar rather than live swapping. Add a "Move to page…" dialog to satisfy SC 2.5.7. Every move is one op in the op log (ADR 0006), so a single Cmd/Ctrl+Z undoes a multi-page move.

**Dark mode.** **[Fact]** pdf.js defaults its page colours to the CSS system colours `Canvas` and `CanvasText`, which follow forced-colours modes ([pdf.js app_options.js](https://github.com/mozilla/pdf.js/blob/master/web/app_options.js)). PDFium has a forced-colours API that recolours only text and paths ([PDFium commit](https://pdfium.googlesource.com/pdfium.git/+/d35a62dfb59e581392e421848740250816993aab%5E%21/samples/pdfium_test.cc)). A naive CSS `invert()` turns photos into negatives ([DEV](https://dev.to/adityads1806/browser-hacks-for-dark-mode-in-pdfs-4igp)).

**[Opinion]** Keep the page appearance separate from the UI theme, with four modes:

- Original (the default, even in dark UI)
- Sepia
- Dark: recolour in the renderer and leave raster images untouched
- High contrast: follow `forced-colors`

Annotation colours never change, because they carry meaning. The page backdrop (the gutter around pages) gets its own design token.

**Perceived speed.**

- **[Fact]** Google's threshold for a "good" Interaction to Next Paint (INP) is ≤ 200 ms at p75. INP replaced FID as a Core Web Vital on 12 March 2024 ([web.dev](https://web.dev/blog/inp-cwv-launch?hl=nl)).
- **[Fact]** pdf.js caps a canvas at about 33.5 MP, and at **5 MP on mobile** ([app_options.js](https://github.com/mozilla/pdf.js/blob/master/web/app_options.js)).
- **[Fact]** Nutrient's Android SDK draws low-resolution images first, sized by device class ([PSPDFKit](https://pspdfkit.com/guides/android/features/low-res-render-api)).

**[Opinion]** `@laminapdf/viewer` should render the way map apps do:

1. Page-size placeholders with the right aspect ratio, so the layout never shifts.
2. A cached low-resolution bitmap stretched to the current zoom.
3. Sharp 256–512 px tiles for the viewport plus a margin, cancelled when the user scrolls or zooms.
4. During a pinch, a CSS transform only; tiles re-render when the gesture ends.

Use optimistic UI, backed by the op log, for annotations and page operations. Show skeletons only in sidebars, never a blank page.

## PDFium paints the pixels, lamina owns the bytes

### Licence filters the field before performance does

Under lamina's licence rule (no AGPL or GPL), the engine field narrows fast:

| Engine | Licence fit | Cold start / size | Memory / mobile risk | Incremental save | Text extraction signal | Role for lamina **[Opinion]** |
|---|---|---|---|---|---|---|
| **PDFium** (via [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries), [pdfium-render](https://github.com/ajrcarey/pdfium-render)) | **Yes**: Apache-2.0 per [EmbedPDF](https://github.com/embedpdf/embed-pdf-viewer); upstream LICENSE not fetched | WASM about **2.7 MB gzipped** **[Report]** ([HN](https://hn.nuxt.dev/item/44126177)); compile costs hundreds of ms ([pdfium.js docs](https://pdfium.js.org/docs/init-library)) | Some WASM builds have a **non-growable heap**, so long documents hit an unrecoverable OOM. Not thread-safe ([pdfium-render](https://github.com/ajrcarey/pdfium-render)) | Saving exists; incremental save not documented | pypdfium2 scores 95% in the py-pdf benchmark ([py-pdf](https://github.com/py-pdf/benchmarks)) | Default renderer, read-only |
| **hayro** | **Yes**: MIT OR Apache-2.0 ([repo](https://github.com/LaurenzV/hayro)) | Pure Rust, CPU-only | Timeouts and OOM still possible on untrusted input | Read-only | n/a | Alternative and CI oracle, the future default |
| **PDF.js** | Probably (Apache-2.0, not verified this session) | Not measured | One canvas per page. **[Fact]** iOS Safari caps a canvas at about 16.7 MP and total canvas memory at 224–384 MB ([pqina](https://pqina.nl/blog/canvas-area-exceeds-the-maximum-limit/); [pqina](https://www.pqina.nl/blog/total-canvas-memory-use-exceeds-the-maximum-limit/)) | Not verified | Not in either benchmark | Differential reference; the only free XFA path on the web |
| **MuPDF / mupdf.js** | **No**: AGPL or commercial ([docs](https://mupdf.readthedocs.io/en/1.27.0/license.html)) | npm package 13.5 MB unpacked ([npm](https://npmjs.com/package/mupdf)) | — | — | PyMuPDF 94% | Prohibited (ADR 0004) |
| **Poppler** | **No**: GPL (not sourced this session) | — | — | — | pdftotext 90% | Excluded |
| **Apryse WebViewer** | Proprietary | — | Blank pages reported past the canvas memory limit ([forum](https://community.apryse.com/t/exceed-canvas-memory-limit-make-pages-document-render-as-blank-pages/10160)) | Not verified | — | Benchmark. Median buyer pays **$25,140/yr** ([Vendr](https://www.vendr.com/marketplace/apryse), **[Estimate]**) |
| **Nutrient (PSPDFKit)** | Proprietary | — | — | Not verified | — | Benchmark. From €5,000 ([Capterra](https://www.capterra.com/p/171174/PSPDFKit-SDK/pricing/)); multi-year lock-in complaints |
| **Foxit SDK** | Proprietary | — | — | — | — | Benchmark. $3,000 on-prem plan listed ([TrustRadius](https://tr-web.dev.trustradius.com/products/foxit-pdf-sdk/pricing)) |
| **pdf_oxide** | Yes, MIT OR Apache-2.0, but forks must not reuse the trademarked name ([repo](https://github.com/yfedoseev/pdf_oxide)) | WASM package exists | — | Claims `SaveOptions::incremental()` | Self-run: 100% pass on 3,830 files (0.8 ms mean) | Extraction baseline to diff against; pre-1.0 |
| **lopdf / pdf-rs / krilla** | Yes (MIT / MIT / MIT+Apache) | — | — | lopdf and pdf-rs: not documented; krilla creates documents only | lopdf 80.2% pass | krilla informs generation (B1b); the others are references only |

### Five conclusions from the comparison

**Cold start and memory.** These favour a tile-based `RenderBackend` more than any particular engine. Every canvas renderer, PDF.js and Apryse included, hits iOS canvas caps on large pages. **[Opinion]** The trait should therefore take `(page, scale, clip rect)` and return a bounded bitmap. The backend should never allocate a full-page canvas. The C1b spike must choose or build a **growable-heap** PDFium build and run one PDFium instance per worker for parallelism.

**Incremental save.** It is lamina's alone. No open-source renderer found documents an append-only save that preserves signatures ([pdfium-render](https://github.com/ajrcarey/pdfium-render); [hayro](https://github.com/LaurenzV/hayro); [krilla](https://github.com/LaurenzV/krilla)). **[Opinion]** Never let PDFium write the document. Keep every renderer read-only and route all writes through the save planner (ADR 0007).

**Text extraction.** Neither benchmark measures reading order. py-pdf's 14-document benchmark leaves its quality metric unexplained ([py-pdf](https://github.com/py-pdf/benchmarks)). pdf_oxide's suite reports *pass rates*: something was extracted, not that it was correct ([pdf_oxide](https://github.com/yfedoseev/pdf_oxide)). Since lamina builds its text layer from its own extraction (B9b) rather than from the renderer, the copy-as-paragraph and column-aware selection features depend on a **ground-truth reading-order corpus** that nobody has published. That is both a risk and a chance to publish a first.

**Spec coverage.** It is uneven:

- **[Fact]** PDFium's XFA support needs V8, and pdfium-binaries ships no V8 build for WASM ([pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)).
- **[Fact]** hayro excludes knockout groups, CMYK blending and 3D ([hayro](https://github.com/LaurenzV/hayro)).

**[Opinion]** "XFA detected, showing fallback" is an acceptable answer, since PDF 2.0 deprecates XFA. Output-intent and overprint files belong in the hayro parity corpus (C1a).

**OCR, the next engine decision.** It should be tiered:

- **ocrs.** MIT/Apache-2.0, runs on the pure-Rust RTen runtime, but an early preview and **Latin script only** ([ocrs](https://github.com/robertknight/ocrs)).
- **PP-OCRv5 mobile.** Apache-2.0 ONNX models of about 20 MiB for detection plus recognition ([oar-ocr models](https://docs.rs/crate/oar-ocr/0.8.1/source/docs/models.md)); these size figures are third-party, not an official PaddleOCR table.
- **Platform OCR.** Apple Vision (`RecognizeDocumentsRequest` on iOS 26+, [Apple docs](https://developer.apple.com/documentation/vision/recognizedocumentsrequest.md)) and ML Kit v2 ([ML Kit](https://developers.google.com/ml-kit/vision/text-recognition/v2/android?hl=fr)) are free and on-device.

Model weights carry their own licences. A third-party ocrs wrapper ships **CC-BY-SA 4.0** weights ([locr](https://github.com/KevRojo/locr)). Two layout models are traps: **DocLayout-YOLO is AGPL-3.0** ([repo](https://github.com/opendatalab/DocLayout-YOLO)), and **Marker's weights use a restrictive modified OpenRAIL-M licence** ([marker](https://github.com/datalab-to/marker)).

## Differentiate on proof, then ship in four narrow phases

### Three differentiators, ranked

The research supports three differentiators. All three are **[Opinion]**, each grounded in sourced grievances.

**First: verifiably private and fast.** "No upload" is now table stakes: Stirling and dozens of indie WASM tools claim it ([dev.to](https://dev.to/pirate_haizei/i-built-a-free-pdf-tool-that-runs-100-in-your-browser-no-uploads-no-servers-2lle)). One write-up notes that "client-side" does not guarantee the page makes no network requests ([dev.to](https://dev.to/rodfreitas/why-your-pdf-tool-shouldnt-need-a-server-building-privacy-first-browser-apps-with-webassembly-1bgh)). The **FBI Denver alert of March 2025** warned that free online converters can deliver malware while still doing the conversion ([Malwarebytes](https://www.malwarebytes.com/blog/news/2025/03/warning-over-free-online-file-converters-that-actually-install-malware)).

lamina should therefore *prove* privacy: a strict CSP on the reader route, an in-app "0 network requests" indicator, offline PWA use, reproducible open-source builds, and the memory-safe parser as a security claim.

**Second: correct by construction.** Edits never break signatures or the original bytes, every change can be undone, and redaction comes with a **verification report**. After applying a redaction, lamina re-parses the output and shows that no text, image or metadata from the redacted regions survives. It refuses to save if anything does. No competitor messaging promoting signature-preserving saves was found, so this is white space or under-researched.

**Third, later: free local intelligence.** That means on-device OCR and semantic search, off by default and never a paid add-on. Embeddings are practical in the browser: about **15 ms per embedding on WebGPU against about 530 ms on WASM** for all-MiniLM-L6-v2, on unstated hardware ([HF benchmark](https://huggingface.co/spaces/Xenova/webgpu-embedding-benchmark/discussions/3)). This is a supporting message, not the lead, because users complain about AI *pricing* rather than ask for AI.

### Monetisation: the Obsidian template

The Obsidian template fits a permissive licence best:

- **[Fact]** The core app has "no account required, no ads, no tracking", and the commercial licence became optional in February 2025. Sync and Publish stay paid ([Obsidian](https://obsidian.md/blog/free-for-work/)).
- **[Fact]** Stirling shows the alternative: free for up to 5 users, Enterprise from $12 per user per month ([pricing](https://stirlingpdf.com/pricing)).

**[Opinion]** Publish a pledge: every on-device feature is free forever, with no account, watermark or page cap. Paid tiers exist only for services that cost money to run: optional end-to-end-encrypted sync (H-series todos), a hosted generation API, support, and enterprise deployment.

### A phased roadmap that fits the existing rounds

| Phase | Scope **[Opinion]** | Maps to | Exit metric |
|---|---|---|---|
| **v0.1 reader** (web + desktop) | Open (file, drag-drop, OS "open with"); tiled render; thumbnails, outline, links; column-aware selection with copy-as-paragraph; find; Original/Sepia/Dark page modes; keyboard map; command palette; password files; repair notice; "0 uploads" indicator; offline PWA. **Out:** annotation, forms, editing, OCR, AI, accounts | Rounds C and D (C3–C7, D2–D7) | Cold start and first-page targets met; crash-free sessions ≥ 99.95%; zero network requests after load |
| **v0.2 trust** | Highlight, note and ink written into the file with appearance streams; AcroForm fill; explicit save (incremental by default) with a constraint banner; **signature validation display**; Zotero/Readwise-compatible export | E1d, E1e, E4b–E4f, E8b–E8g, E10g | Annotations round-trip in Acrobat, pdf.js and PDFium; no silent writes |
| **v0.3 organise and protect** | Page organiser; merge and split; compress to a target size; true redaction with mark → review → apply and the verification report; two clearly named signing paths | E2, E12, E16b–c, E10c–d, E10j | Redaction leak tests pass (E12f); signed files keep valid signatures after edits |
| **v0.4+ intelligence and reach** | Local OCR (platform APIs on native, ocrs or PP-OCR in WASM); tagged-PDF reflow, then heuristic reflow; local semantic search; flat-form field detection; citation previews; mobile apps | E13c–d, E9d, B8a plus new reflow and mobile todos | Reflow beats Liquid Mode on a public, tagged corpus |

The existing plans' targets are looser than the research team's suggested ones, and the difference should be settled explicitly:

| Metric | Existing plan | Research team's suggestion |
|---|---|---|
| Desktop cold start to window | ≤ 1 s (`docs/plans/desktop-app.md`) | < 300 ms to interactive |
| Desktop installer | ≤ 40 MB | < 15 MB |
| Web first page (warm cache, mid-range laptop) | ≤ 700 ms (`docs/plans/web-app.md`) | 10 MB PDF in < 500 ms |

**[Opinion]** Keep the plan's numbers as release gates and the research team's as published "stretch" goals. Sumatra-class speed is the bar users quote.

**[Fact]** Instabug/Luciq's vendor dataset gives a median crash-free session rate of 99.95% across apps; apps rated under three stars have a median of 99.82% ([Luciq](https://www.luciq.ai/blog/benchmarking-crash-free-sessions-for-mobile-apps-whats-a-good-crash-free-rate)). That makes 99.95% a sensible floor.

### Mobile stays an open question, with a clear spike

**[Opinion]** Treat mobile in two steps. First, the responsive PWA from Round C covers casual phone reading with the bottom-bar layout above. Then, before any native work, run a **spike comparing Tauri 2 mobile with UniFFI plus native UI (SwiftUI and Compose)** on four criteria:

- conflicts with the native selection menu (Wikimedia's iOS finding)
- Apple Pencil latency and `pointerType` handling
- share-sheet and Files integration
- access to Vision and ML Kit OCR

PDFium binaries already exist for iOS and Android ([pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)), and the core crates already check for iOS and Android targets. The risk is therefore UI shell fidelity, not the engine.

## Recommendations mapped to lamina's architecture

All entries below are **[Opinion]**.

| # | Recommendation | Where it lands |
|---|---|---|
| R1 | Inverted-L chrome: top bar, collapsible left rail, docked right panel shown on selection, focus toggle; no free-floating palettes over the page | `@laminapdf/ui`, C4a |
| R2 | Modes (Read / Comment / Fill & Sign / Organise / Edit), each swapping one secondary toolbar; Read is the default and minimal | `@laminapdf/ui`, C4a; editor E1a |
| R3 | One command registry (id, title, keywords, shortcut, `enabled(ctx)`, typed arguments) feeding the cmdk palette, native menus, context menus and the shortcut help | New todo; C4a, D4b, C7a |
| R4 | Selection: the viewer emits rects; the UI shows a desktop mini toolbar and a mobile bottom action bar; the native OS menu is never covered | `@laminapdf/viewer` C5a, `@laminapdf/ui` |
| R5 | Column-aware selection with "Copy as paragraph" by default and "Copy exact" as the alternative, built on lamina's extraction plus a ground-truth reading-order corpus | B9b, B9c, C5a |
| R6 | `RenderBackend` takes a tile (page, scale, clip) and returns a bounded bitmap; low-res first, then sharp tiles, with cancellation; total bitmap memory capped, 5 MP on mobile | C2a (ADR 0004), C2b, C3b, C3c |
| R7 | PDFium is read-only; pick a growable-heap WASM build; one instance per worker; check upstream licence notices | C1b, C2a |
| R8 | Page appearance modes (Original, Sepia, Dark recoloured in the renderer, High contrast), separate from UI theme tokens; annotation colours never change | C7a, `lamina-render` |
| R9 | Verifiable privacy: CSP `connect-src` limited to self, an in-app network indicator, offline PWA, no document-content analytics | C4c, C7b, ADR 0011 |
| R10 | Explicit save only; incremental by default; the save-constraint banner explains what saving will do to signatures | E1d, E1e, ADR 0007 |
| R11 | Annotations and form values written into the file with appearance streams; Zotero/Readwise-friendly highlight export | E4, E5d, E8d |
| R12 | Redaction as a separate tool with mark → review → apply plus a verification report that blocks saving on any leak | E12b–E12f |
| R13 | Two named signing actions with plain-language explanations and a validation badge on open | E10c, E10d, E10g, E10j |
| R14 | Organiser grid: multi-select, insertion bar, keyboard moves, a "Move to page…" dialog, and one undo step per batch | E2h, ADR 0006 |
| R15 | Two-tier reflow (structure tree, then heuristic labelled "Approximate"); figures, tables and maths shown as page crops | New todo after B8a |
| R16 | Tiered OCR behind one trait, with model weights' licences audited separately; no AGPL or OpenRAIL weights in the core | E13a, E13d |
| R17 | Mobile spike: Tauri 2 mobile vs UniFFI with native UI, before any native app work | New todo; architecture open question 4 |
| R18 | Publish a no-hostile-monetisation pledge and the benchmark targets, including "opens faster than Acrobat on the same file" | Docs site B7a, launch C7c |
| R19 | Opt-in, content-free usage telemetry to test the task-split hypothesis | ADR 0011 |

## Several primary sources were unreachable, and the gaps matter

These sites were blocked or did not resolve from the research sessions:

- adobe.com and community.adobe.com (only search snippets were available)
- business.adobe.com
- apryse.com
- webaim.org
- www.w3.org (the WCAG 2.2 and WAI-ARIA APG pages)
- web.dev
- pdfa.org
- github.blog (GitHub's accessible sortable-list case study)
- uxmatters.com (Hoober's original data)
- docs.rs
- embedpdf.com

Some content was not retrievable at all: Reddit (r/pdf, r/macapps, r/ipad, r/academia), App Store and Play Store reviews, and 2025–2026 tech-press reviews. Claims about these sites therefore rest on search-result summaries or secondary sources, and should be re-checked before they go into an ADR.

Several gaps are substantive:

- No quantitative benchmarks compare apps on cold start, memory or battery.
- No public task-frequency data exists.
- No 2022–2026 data on researchers' devices was found.
- Upstream PDFium's LICENSE file was not fetched.
- The licences of PDF.js, Poppler and the newer dnd-kit packages were not verified.
- The licences of Docling's model weights were not checked.
- No sourced praise of GoodNotes's stylus feel or LiquidText's workspace was found.

lamina's own spikes (C1a, C1b, C3c) are where the missing engine numbers should come from.

## Conclusion

The research shifts the question from "which features does lamina need to catch Acrobat?" to "which promises can lamina make that incumbents cannot afford to keep?" Adobe cannot drop the AI tier. Preview cannot become strict about writes without breaking its simplicity. Web tools cannot stop uploading without killing their task-rationing revenue. Stirling cannot drop its toolbox UI without abandoning its server model. lamina's architecture choices made those promises cheap, and the UX plan's job is to surface them: the network indicator, the save-constraint banner, the redaction verification report and the published benchmark.

The non-obvious risk is text, not pixels. PDFium makes rendering a solved problem for v0.1, but the frictions users feel most are wrong copy order, missing reflow, and annotations that don't survive other apps. All of them depend on reading-order and structure work that no benchmark measures yet. Building the ground-truth corpus early, alongside B9 and B8a, is the cheapest way to turn lamina's weakest unknown into its most visible, demo-able advantage.
