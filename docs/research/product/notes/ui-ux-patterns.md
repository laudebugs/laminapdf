# UI/UX architecture patterns for a fast, simple cross-platform PDF app (2026)

Scope: design guidance for lamina's UI plan: React 19 chrome (`@lamina/ui`) over a framework-free viewer (`@lamina/viewer`), shipped as a web app and a Tauri 2 desktop app, with a responsive mobile layout.

Method note: WebFetch could not reach www.w3.org, web.dev, pdfa.org, github.blog or uxmatters.com from this session (DNS blocked by the session's network allowlist). Claims about those sites come from search-result summaries or secondary sources, and are marked as such. github.com was reachable, so the cmdk README and the pdf.js `app_options.js` were read directly. Everything under "Inferences" is opinion or recommendation, not sourced fact.

## Progressive disclosure and canvas-first layouts (Linear, Figma, Acrobat 2023+, Preview/PDF Expert/Arc; ribbons vs contextual toolbars)

### Takeaway
The evidence supports stable, docked chrome around a large canvas, with a "minimize UI" escape hatch and contextual controls that appear on selection. Floating panels that cover content were rejected by users in both Figma UI3 and Acrobat's 2023 floating quick-tools widget. The ribbon research (Office 2007) shows that discoverability problems come from command density, and its contextual tabs (shown only when relevant) are the part worth copying.

### Cited Findings
- Linear's 2024 redesign focused on the "inverted L": the global chrome (sidebar and top header) that controls the content in the main view. Its goals were less visual noise and better navigation through changes to the sidebar, tabs, headers and panels. — [Linear: How we redesigned the Linear UI](https://linear.app/blog/how-we-redesigned-the-linear-ui); [Linear changelog 2024-03-20](https://linear.app/changelog/2024-03-20-new-linear-ui)
- Linear has published a later refresh ("since the last major redesign in 2024, Linear has evolved considerably"). — [Linear: Behind the latest design refresh](https://linear.app/blog/behind-the-latest-design-refresh)
- Third-party analyses describe Linear's Cmd+K as the unified entry point for all actions. This is not from Linear's own documentation. — [blakecrosley.com guide](https://blakecrosley.com/de/guides/design/linear)
- Figma UI3 launched with floating panels, then reattached the navigation and properties panels in Figma Design after user feedback. Panels still float in "Minimize UI" mode, in Slides grid view and in FigJam. — [Figma 2024: we shipped it, you shaped it](https://www.figma.com/blog/figma-2024-we-shipped-it-you-shaped-it/); [Our approach to designing UI3](https://www.figma.com/blog/our-approach-to-designing-ui3/)
- Figma's Minimize UI (Shift+\) collapses both side panels. When the user selects something on the canvas, the properties panel temporarily reopens. — [Figma Help: Navigating UI3](https://help.figma.com/hc/articles/23954856027159)
- A forum post relays the team saying floating panels would not return to UI3 "due to the overwhelming amount of negative feedback". This is a user's unofficial relay of the team's comments. — [Figma forum: shortcut to bring back floating panels](https://forum.figma.com/t/shortcut-to-bring-back-floating-panels-in-ui3/88458); commentary: [uxdesign.cc: Figma scrapes UI3's floating panels](https://uxdesign.cc/figma-scrapes-ui3s-floating-panels-44303803c1ad)
- The 2023 "new Acrobat" has a global bar, with PDF tools on the left (an "All Tools" panel as a single access point) and file actions (search, save, print, share) on the right. A right rail holds bookmarks, view and page navigation at the top, and zoom and view options at the bottom. — [Adobe: Learn about the new Acrobat](https://helpx.adobe.com/be_en/acrobat/learn-new-acrobat.html); [Adobe Experience League: New workspace experience](https://experienceleague.adobe.com/en/docs/document-cloud-learn/acrobat-learning/getting-started/new-workspace)
- Acrobat's floating "quick tools" widget drew user complaints because it covered page content. Adobe made it movable in 23.06.20320. Staff said the left-side tools panel cannot be swapped to the right, and users could revert with "Disable New Acrobat". — [Adobe Community thread](https://community.adobe.com/t5/acrobat-discussions/how-do-i-move-remove-the-quick-actions-toolbar-from-acrobat-pro-2022-003-xxxx/m-p/13946883/highlight/true); [Adobe Community: toolbar switched sides](https://community.adobe.com/t5/acrobat-discussions/toolbar-has-switched-sides-from-left-to-right/m-p/13798436/highlight/true)
- Office ribbon background (secondary sources):
  - Office had about 2,500 features, and before 2007 a typical user used only 5–20% of them. Most feature requests were for features that already existed. — [Microsoft archive blog](https://learn.microsoft.com/vi-vn/archive/blogs/cbortlik/so-thats-why-office-2007-looks-different)
  - In the telemetry about 80% of saves came from the toolbar icon and about 20% from Ctrl+S. This is from a secondary blog account of Jensen Harris's talk. — [GNOME blog (Dave Neary)](https://blogs.gnome.org/bolsh/?p=582)
  - The products had "outgrown" menus and toolbars, and contextual tabs appear only when relevant. — [Thurrott: From the Vaults: The Office Ribbon](https://www.thurrott.com/office/161324/vaults-office-ribbon); [Gihyo Pacific Connection](https://gihyo.jp/admin/serial/01/pacific/200705)
- Microsoft's Windows UX guidance warns that progressive disclosure must feel expected, because controls that appear and disappear unexpectedly make an interface feel unstable. — [Microsoft Learn: Progressive Disclosure Controls](https://learn.microsoft.com/sk-sk/windows/win32/uxguide/ctrl-progressive-disclosure-controls)
- The ribbon-usage statistic above is a lesson about discoverability: users click visible affordances far more than they use shortcuts. — same [GNOME blog](https://blogs.gnome.org/bolsh/?p=582)

### Inferences
- (Opinion) Use an inverted-L for desktop and web: a slim top bar (file actions, mode switcher, search, Cmd/Ctrl+K entry) and a collapsible left rail (thumbnails, outline, bookmarks). The canvas fills the rest. Tool-specific properties go in a docked right panel that opens only when an annotation or object is selected, as in Figma's Minimize UI. Avoid free-floating tool palettes that cover the page; both Figma and Acrobat paid for that choice.
- (Opinion) Use modes ("Read / Comment / Edit / Organise / Fill & Sign") as the progressive-disclosure layer. Each mode swaps a single secondary toolbar, which is a lighter version of the ribbon's contextual tabs. Keep a visible button for every frequent command, because the Office telemetry shows that shortcuts and palettes do not replace visible affordances.
- (Opinion) Provide a "focus/minimize UI" toggle (one shortcut, e.g. Shift+\ or F11-like) that hides both rails. A selection on the canvas temporarily brings back the contextual panel.

### Gaps
- No NN/g primary articles were fetched (nngroup.com did not appear in results). NN/g claims on progressive disclosure, ribbons and contextual menus are not cited directly here.
- Apple Preview, PDF Expert and Arc toolbar structure: no 2023–2026 primary sources found in this pass.
- No quantitative A/B data on ribbon vs contextual toolbar for PDF apps was found.

## Floating contextual menus on text selection (placement, iOS/Android collision, accessibility)

### Takeaway
On touch devices the OS or browser already shows its own selection menu (Android floating ActionMode, iOS UIEditMenuInteraction, the browser's native menu on the web), and it cannot be reliably suppressed or repositioned from web content. A custom floating menu on mobile web will therefore compete with it. Desktop can use a Medium/Acrobat-style mini toolbar. Mobile should put selection actions in a bottom bar or bottom sheet and leave the native menu alone.

### Cited Findings
- Android M introduced a floating toolbar for text selection, which is the default in TextView. Apps customise it through `ActionMode.TYPE_FLOATING`. — [Android ActionMode reference](https://developer.android.com/reference/android/view/ActionMode.html); [Android Police (2015)](https://www.androidpolice.com/2015/06/03/android-m-feature-spotlight-faster-text-selection-and-a-floating-toolbar-for-text-actions/)
- Cross-platform toolkits use each platform's own selection menu: the floating ActionMode on Android, UIEditMenuInteraction or UIMenuController on iOS, and the browser's native menu on the web. — [native_adaptive_toolbox (pub.dev)](https://pub.dev/packages/native_adaptive_toolbox)
- Wikimedia's mobile editor found that the native selection menu covered its editor toolbar. On iOS, scrolling hides the copy/paste menu and there is no way to bring it back programmatically. — [Wikimedia Phabricator T202723](https://phabricator.wikimedia.org/T202723)
- WCAG 2.2 (AA) applies to floating UI:
  - 2.4.11 Focus Not Obscured (Minimum): a focused element must not be completely hidden by author content such as sticky bars.
  - 2.5.8 Target Size (Minimum): at least 24×24 CSS px, or enough spacing.
  - 2.5.7 Dragging Movements: a single-pointer alternative to dragging.
  - 44×44 is the AAA criterion 2.5.5.
  — [W3C: What's new in WCAG 2.2](https://w3.org/WAI/standards-guidelines/wcag/new-in-22/) (via search summary); [TetraLogical: What's new in WCAG 2.2](https://tetralogical.com/blog/2023/10/05/whats-new-wcag-2.2); [Deque: WCAG 2.2](https://dequeuniversity.com/resources/wcag-2.2). Some Deque materials number these criteria differently; use the W3C numbering.

### Inferences
- (Opinion) Placement rule for desktop: anchor the mini toolbar above the selection's first line rect, flip below when it would collide with the viewport top or the top bar, and clamp horizontally to the viewport. The `@floating-ui/dom` flip/shift middleware is the usual implementation (it is MIT-licensed, but its licence was not verified in this pass). Do not move the menu during a drag-select; show it on `pointerup` or after `selectionchange` settles.
- (Opinion) Mobile: do not draw a floating menu over the native one. Show a contextual bottom action bar (Highlight / Underline / Comment / Copy / More) when a selection exists, inside the safe area. On iOS, scrolling dismisses the native menu, which reinforces this choice.
- (Opinion) Accessibility:
  - The mini toolbar should be `role="toolbar"` with roving tabindex and arrow-key navigation.
  - It must be reachable from the keyboard after a keyboard selection, e.g. by Shift+F10 or a documented shortcut, and Escape returns focus to the text.
  - Every action must also exist in the menu bar or palette, so the floating menu is never the only path.
  - Announce "Selection actions available" with a polite live region rather than moving focus automatically.
- (Opinion) Because the viewer is framework-free, the viewer should emit a selection event with client rects. `@lamina/ui` owns the menu, which keeps the viewer headless.

### Gaps
- Could not fetch the WAI-ARIA APG toolbar or menu patterns (w3.org blocked) or the Apple HIG edit-menu page. No primary source was found on suppressing the iOS Safari or WKWebView selection menu from web content. The usual approach is `-webkit-touch-callout: none` together with a custom selection layer; that is unverified.
- No placement guidance from Acrobat's quick-actions documentation was found.

## Command palettes (Cmd/Ctrl+K): design, libraries, licences, accessibility

### Takeaway
cmdk (MIT) is the default choice for React: unstyled, a combobox-style accessible list, async items, and a pluggable filter with `shouldFilter={false}` for bringing your own ranking. kbar (MIT) is an alternative but looks less active (last push March 2024). Argument entry ("extract pages 4-8") is not built in to either library and has to be designed as nested pages or parsed input.

### Cited Findings
cmdk:
- MIT-licensed; requires React 18+ (it uses `useId` and `useSyncExternalStore`); unstyled. — [cmdk GitHub README](https://github.com/pacocoursey/cmdk)
- Supports a custom `filter(value, search, keywords) → rank`, async results with `Command.Loading`, and `shouldFilter={false}` for custom filtering or virtualization. — [cmdk GitHub README](https://github.com/pacocoursey/cmdk)
- Has no built-in "pages" feature; the README shows nested pages through a page stack held in component state. — [cmdk GitHub README](https://github.com/pacocoursey/cmdk)
- Labelling, aria attributes and DOM order were tested with VoiceOver and Chrome DevTools, and the dialog variant uses Radix Dialog. — [cmdk GitHub README](https://github.com/pacocoursey/cmdk); [npm: cmdk](https://www.npmjs.com/package/cmdk)
- React 19 support is listed by third-party directories. — [openapps.pro](https://openapps.pro/packages/cmdk)

kbar:
- MIT, by timc1; npm version 0.1.0-beta.45; last recorded push March 2024 (Ecosyste.ms). — [npm: kbar](https://npmjs.com/package/kbar); [Ecosyste.ms](https://awesome.ecosyste.ms/projects/36282)

### Inferences
- (Opinion) Use cmdk (MIT) for a single command registry shared by the menu bar (Tauri native menu), toolbar, context menus, the palette and the shortcut help.
- (Opinion) Each command has an id, title, keywords, a shortcut, an `enabled(ctx)` check and optional typed arguments.
- (Opinion) Ranking: boost by recency and frequency, then by match score. Show "Recent" when the query is empty. Show the shortcut chip on every row, which is the main way shortcuts become discoverable.
- (Opinion) Argument entry: when a command declares arguments (`extract pages <range>`), selecting it pushes a page whose input parses a page range (`4-8, 10`). Validation errors are shown inline. Also accept direct "verb + args" typing (e.g. `extract 4-8`) parsed in a `filter` step with `shouldFilter={false}`.
- (Opinion) Accessibility: follow the APG combobox pattern. Focus stays in the input with `aria-activedescendant` pointing at the active option; Up/Down moves; Enter executes; Escape closes and restores focus to the previous element. Announce the result count politely.

### Gaps
- The APG combobox page (w3.org) could not be fetched, so the specifics above come from general knowledge and should be checked there.
- cmdk's default scoring algorithm is not documented in the README (it was historically `command-score`; unverified). No current cmdk version number was found.

## Mobile thumb-zone ergonomics: Hoober, bottom bars and sheets, gesture conflicts, safe areas

### Takeaway
Hoober's observational data shows that people change how they hold the phone, and his later book argues that touch accuracy is best at the centre and that "thumb zone" folklore is often wrong. Put primary actions in a bottom bar, use sheets with detents for secondary panels (thumbnails, outline, annotation properties), keep the page-canvas gestures (pan, pinch, double-tap) owned by the viewer, and pad all chrome with `env(safe-area-inset-*)`.

### Cited Findings
- Hoober's 2013 column "How Do Users Really Hold Mobile Devices?" observed 1,333 people. A secondary summary reports about 49% one-handed, 36% cradled (held in one hand, operated with the other) and 15% two-handed. These figures are second-hand; the original page could not be fetched. — [UXmatters Mobile Matters](https://uxmatters.com/columns/mobile-matters); [woshipm summary](https://www.woshipm.com/ucd/57973.html)
- Hoober's book *Touch Design for Mobile Interfaces* (Smashing, 2022) covers touch accuracy, a "center-out" preference, how fingers obscure content, and designing by zones. It argues that popular notions of touch accuracy and preferred regions are wrong. — [Smashing: Touch Design for Mobile Interfaces](https://www.smashingmagazine.com/printed-books/touch-design-for-mobile-interfaces/)
- Apple HIG sheets support the large detent by default. Adding medium lets the sheet rest at both heights; specifying only medium prevents a full-height expansion. — [Apple HIG: Sheets](https://developer-mdn.apple.com/design/human-interface-guidelines/components/presentation/sheets); [sarunw: UISheetPresentationController detents](https://sarunw.com/posts/bottom-sheet-in-ios-15-with-uisheetpresentationcontroller/)
- A third-party library documents Material 3 values for bottom sheets: a 32×4 dp drag handle and a 16 dp corner radius. This is not Google's own documentation. — [react-native-true-sheet docs](https://sheet.lodev09.com/next/reference/configuration)
- On iOS, `env(safe-area-inset-*)` is 0 unless the viewport meta sets `viewport-fit=cover`. Android WebView may report non-zero insets regardless. A common pattern is `max(env(safe-area-inset-bottom), Npx)` padding on the bottom bar. — [OpenHarmony web safe-area doc](https://gitcode.com/openharmony/docs/blob/OpenHarmony-5.0.2-Release/en/application-dev/web/web-safe-area-insets.md); [WebKit bug 272779](https://bugs.webkit.org/show_bug.cgi?id=272779)
- Native PDF views use pinch, quick-scale and double-tap gestures for zoom, so these are the platform conventions users expect. — [AndroidX PdfView](https://developer.android.com/reference/androidx/pdf/view/PdfView?authuser=4)
- WCAG 2.5.8 sets a minimum target of 24×24 CSS px (AA); 2.5.5 is 44×44 (AAA). — [TetraLogical](https://tetralogical.com/blog/2023/10/05/whats-new-wcag-2.2)

### Inferences
- (Opinion) Mobile layout:
  - A top app bar holds only the title, back and overflow; it auto-hides on scroll and reappears on a tap on the canvas.
  - A bottom bar holds four or five primary actions: Search, Annotate, Pages, Reader mode, More.
  - Thumbnails, outline and annotation properties live in a bottom sheet with medium and large detents.
  - Targets are at least 44 px.
- (Opinion) Gesture ownership:
  - The viewer sets `touch-action: none` on the canvas and implements pan, pinch and double-tap itself, so the browser's page zoom doesn't fight it.
  - Edge-swipes are left to the OS (back gestures).
  - Long-press is selection, and drawing tools need an explicit mode (or Apple Pencil/stylus via `pointerType`) so a one-finger pan never draws by accident.
  - Inside sheets, the sheet drag only starts from the handle or header when the content is scrolled to the top.
- (Opinion) Set `viewport-fit=cover` and pad the bars with safe-area insets. Don't use `100vh`; prefer `dvh`/`svh`.

### Gaps
- Material 3 primary bottom-sheet guidance (m3.material.io) was not retrieved. MDN `touch-action` was not fetched. Hoober's original 2013 numbers were not verified first-hand.

## Reflow / reader mode for fixed-layout PDFs

### Takeaway
Every reflow implementation either uses the tag tree (when present and correct) or infers structure: Acrobat desktop Reflow, Liquid Mode (Adobe's Sensei AI), Kindle's conversion and k2pdfopt. Quality is bounded by reading-order correctness. Tables, images and complex layouts are the usual failure cases, and already-accessible documents may read better in the original layout. The PDF Association's "Deriving HTML from PDF" (v1.0, June 2019; a 2nd edition is in development) defines a tag-tree-to-HTML algorithm for tagged ISO 32000-2 files. It is the obvious standards-based path for the tagged case.

### Cited Findings
Acrobat Liquid Mode and Reflow:
- Liquid Mode (2020, Acrobat Reader iOS/Android) uses Adobe Sensei AI to identify headings, paragraphs and lists and rebuild the layout for small screens. The original file is unchanged. — [Adobe Help: Use Liquid Mode](https://helpx.adobe.com/acrobat/mobile/view-manage-files/liquid-mode.html); [TechCrunch (2020)](https://techcrunch.com/2020/09/23/adobes-liquid-mode-uses-ai-to-automatically-redesign-pdfs-for-mobile-devices); [Adobe blog (2020)](https://blog.adobe.com/en/publish/2020/12/10/adobe-delivers-enhanced-pdf-reading-experience-for-accessibility)
- Liquid Mode does not apply to files over 200 pages. Adobe notes that screen-reader users may find the original layout better for documents already made accessible. — [Adobe Help: Use Liquid Mode](https://helpx.adobe.com/acrobat/mobile/view-manage-files/liquid-mode.html); [Adobe blog](https://blog.adobe.com/en/publish/2020/12/10/adobe-delivers-enhanced-pdf-reading-experience-for-accessibility)
- Desktop Acrobat Reflow (View > Zoom > Reflow) linearises content by order and temporarily tags untagged files. Forms, comments and page headers or footers don't reflow, and reflow cannot correct a bad reading order. — [Fondazione LIA test](https://www.fondazionelia.org/en/research-and-development/reflowing-pdfs-with-acrobat-reader-we-have-tested-the-new-feature/); [w3c-wai-ig thread (2024)](https://lists.w3.org/Archives/Public/w3c-wai-ig/2024AprJun/0207.html)
- Fondazione LIA's 2024 test found the reflow order did not always match the reading order. — [Fondazione LIA](https://www.fondazionelia.org/en/research-and-development/reflowing-pdfs-with-acrobat-reader-we-have-tested-the-new-feature/)

Kindle and k2pdfopt:
- Amazon's PDF-to-Kindle conversion is "hit or miss" with images and tables and works best for text-heavy files. — [The eBook Reader blog](https://blog.the-ebook-reader.com/?p=14292)
- k2pdfopt is a free, open-source tool that re-rasterises PDF or DjVu (especially two-column documents) for e-readers. It has Tesseract OCR, and users report it needs flag tuning; NYPL Labs found spottier results on complex layouts. — [k2pdfopt man page](https://manpages.debian.org/bullseye/k2pdfopt/k2pdfopt.1); [MobileRead thread](https://www.mobileread.com/forums/showthread.php?p=3965080); [JSTOR Labs / NYPL](https://labs.jstor.org/blog/nypl-open-e-book-page-for-2)
- k2pdfopt's licence was not verified here (believed GPL, which would be unusable for lamina).

Other apps and the PDF Association spec:
- Other apps with reflow modes include PDF Reader Pro (Text Reflow) and GoodReader ("experimental" PDF Reflow). — [PDF Reader Pro help](https://www.pdfreaderpro.com/windows/help/how-to-read-pdfs-with-text-reflow); [GoodReader manual](https://goodreader.com/user-manual/pdf-reflow)
- "Deriving HTML from PDF" is a PDF Association usage specification for tagged ISO 32000-2 files that defines an algorithm for converting them to HTML. v1.0 was published in June 2019 (lead author Roman Toda), a v1.1 draft circulated in 2023, and a working group is developing a 2nd edition. — [PDF Association: Deriving HTML from PDF](https://www.pdfa.org/resource/deriving-html-from-pdf); [Deriving HTML from PDF TWG](https://pdfa.org/community/deriving-html-from-pdf-twg/); [Lessons learned presentation](https://pdfa.org/presentation/deriving-html-from-pdf-lessons-learned/)
- BFO ships a commercial `HtmlDerivation` class implementing the spec. — [BFO HtmlDerivation API](https://bfo.com/products/pdf/docs/api/org/faceless/pdf2/HtmlDerivation.html)
- A "v1.2 (September 2026)" reference in a PHP package README could not be confirmed against a PDF Association source. — [packagist pdftoolkit/pdf-to-html](https://packagist.org/packages/pdftoolkit/pdf-to-html)

### Inferences
- (Opinion) Two-tier reader mode:
  1. If the document is tagged (`/MarkInfo /Marked true` plus a StructTreeRoot), derive HTML from the structure tree per "Deriving HTML from PDF". This is ISO 32000-2 §14.8 territory; read the spec section before implementing.
  2. Otherwise, run a heuristic layout analysis (columns, reading order, headings by font size, header/footer removal across pages) and label the result "Approximate".
- (Opinion) Never reflow figures, tables or math. Render them as cropped page images with alt text from `/Alt` when present. Always offer a one-tap "back to page" that maps the scroll position to the original page, as Liquid Mode does.
- (Opinion) Tables, multi-column magazines, footnotes and formulas are the known failure modes. Consider a per-block "show original" toggle, and treat reflow as read-only.
- (Opinion) Reflow output is HTML in the chrome layer. It should be a core-crate output (structure → semantic blocks), with `@lamina/ui` rendering it, which keeps the core bytes-in/bytes-out.

### Gaps
- Microsoft Edge's PDF reader has no documented reflow mode found in this pass. Nothing was found on PDF Expert's reflow or ReadEra (no confirmation of a reflow mode).
- The content of the 2nd-edition "Deriving HTML" draft and ISO 32000-2's derivation clauses was not retrieved (pdfa.org blocked).

## Thumbnail grid page organisers: multi-select, drag-and-drop, keyboard reordering, undo

### Takeaway
dnd-kit (MIT, at least `@dnd-kit/accessibility`) provides keyboard sensors and live-region announcements out of the box. WCAG 2.2 SC 2.5.7 also requires a non-drag alternative: Move-up/down buttons, "Move to…" with a page number, or cut and paste.

### Cited Findings
- The current dnd-kit (`@dnd-kit/dom`) includes an Accessibility plugin by default. It manages ARIA attributes, supplies screen-reader instructions and announces drag events through a live region, with customisable announcements and a 500 ms default debounce. — [dnd kit docs: Accessibility](https://dndkit.com/guides/accessibility); [dnd kit: accessibility plugin](https://dndkit.com/extend/plugins/accessibility)
- The documented dnd-kit keyboard controls are arrows to move, Shift+arrows for faster movement, Escape to cancel, and Tab to drop and move focus. — [dnd kit docs: Accessibility](https://dndkit.com/guides/accessibility)
- The legacy `@dnd-kit/core` enables a keyboard sensor by default, and its announcements ship in English only. — [dnd kit legacy accessibility guide](https://dndkit.com/legacy/guides/accessibility)
- `@dnd-kit/accessibility` is MIT (v3.1.0). The licence of `@dnd-kit/dom` and `@dnd-kit/react` was not confirmed in this pass. — [npm @dnd-kit/accessibility](https://www.npmjs.org/package/@dnd-kit/accessibility)
- GitHub published (July 2024) its exploration of an accessible sortable list. The article could not be fetched; it is the most relevant case study. — [GitHub blog: accessible sortable list](https://github.blog/2024-07-09-exploring-the-challenges-in-creating-an-accessible-sortable-list-drag-and-drop/)
- WCAG 2.5.7 Dragging Movements (AA): anything done by dragging must be achievable with a single pointer without dragging, e.g. with move buttons. — [TetraLogical](https://tetralogical.com/blog/2023/10/05/whats-new-wcag-2.2); [W3C: What's new in WCAG 2.2](https://w3.org/WAI/standards-guidelines/wcag/new-in-22/)

### Inferences
- (Opinion) Grid semantics: `role="grid"` or a listbox with `aria-multiselectable="true"`.
  - Arrows move focus; Space toggles selection; Shift+arrows and Shift+click extend a range; Cmd/Ctrl+click toggles.
  - Cmd/Ctrl+A selects all; Delete removes; R/Shift+R rotates.
  - Alt/Option+arrows moves the selection by one slot; Cmd/Ctrl+X then V at the focused gap moves pages.
  - "Move to page…" opens a dialog as the WCAG 2.5.7 alternative.
- (Opinion) Drag physics: lift with a small scale or shadow, stack multi-selected pages under the cursor with a count badge, auto-scroll near the edges, and show an insertion bar between items rather than swapping live, which is calmer for large grids. Animate the rest of the grid with FLIP transforms.
- (Opinion) Every organiser action is an op in the op log (ADR 0006), so a single Cmd/Ctrl+Z undoes a multi-page move. Show a toast with Undo after destructive actions such as delete.
- (Opinion) Virtualise the grid for 1,000+ page documents. Thumbnails come from a low-res render queue prioritised by visibility.

### Gaps
- The WAI-ARIA APG grid and listbox patterns were not fetched. The GitHub blog's specific conclusions are unknown.

## Performance perception: low-res-first, optimistic UI, 100 ms / INP, tile rendering

### Takeaway
Target INP ≤ 200 ms at p75 (Google's "good" threshold, a Core Web Vital since 12 March 2024), and acknowledge every input within one frame. Render pages map-style: show a scaled-up cached or low-res bitmap immediately, then sharp tiles for the visible region. pdf.js and PSPDFKit/Nutrient both do this, and pdf.js caps canvas size, especially on mobile.

### Cited Findings
- INP thresholds are ≤ 200 ms good, ≤ 500 ms needs improvement and > 500 ms poor, measured at p75. INP replaced FID as a Core Web Vital on 12 March 2024 and measures all interactions, not just the first. — [web.dev: INP is officially a Core Web Vital](https://web.dev/blog/inp-cwv-launch?hl=nl); [Sentry: Web Vitals concepts](https://docs.sentry.io/product/insights/web-vitals/web-vitals-concepts); [Cloudflare blog](https://blog.cloudflare.com/inp-get-ready-for-the-new-core-web-vital/)
- pdf.js viewer defaults (`web/app_options.js`):
  - `maxCanvasPixels = 2**25` (about 33.5 MP), lowered to 5,242,880 on mobile ("Limit canvas size to 5 mega-pixels on mobile").
  - `maxCanvasDim = 32767`; `enableDetailCanvas = true`.
  — [pdf.js app_options.js](https://github.com/mozilla/pdf.js/blob/master/web/app_options.js)
- pdf.js's detail-canvas change renders a high-resolution region larger than the visible area, limited by the maximum canvas size. It prioritises the full CSS-zoomed canvas so users don't see white while the high-resolution canvas is still rendering. — [pdf.js PR #19128](https://redirect.github.com/mozilla/pdf.js/pull/19128)
- EmbedPDF's tiling plugin shows a fast low-resolution preview first, then fills in high-resolution tiles progressively. — [EmbedPDF tiling plugin](https://www.embedpdf.com/docs/react/headless/plugins/plugin-tiling)
- EmbedPDF's licence was not verified; check it before borrowing code.
- PSPDFKit/Nutrient on Android draws low-res images for pages about to be displayed, then switches to high-res for visible pages. The low-res size is chosen by device class. — [PSPDFKit: low-res render API](https://pspdfkit.com/guides/android/features/low-res-render-api)
- A PDFium-wrapper write-up recommends scaling the previous cached bitmap to the new zoom as a placeholder ("looks soft for 100–200 ms and feels instant"). It also notes that whole-page renders at high zoom of large sheets become multi-hundred-MB allocations, which is the case for tiling. This is a third-party, indirect source. — [loslab: PDFium render cache & zoom](https://blog.loslab.com/en-us/pdfium-component/pdfium-component-render-cache-zoom-performance.html)
- Apple's sample ZoomingPDFViewer uses a tiled scroll view (CATiledLayer) for zoomed PDF. — [Apple ZoomingPDFViewer sample](https://developer.apple.com/library/archive/samplecode/ZoomingPDFViewer/Listings/ZoomingPDFViewerSwift_ZoomingPDFViewer_TiledPDFScrollView_swift.html)

### Inferences
- (Opinion) Render pipeline for `@lamina/viewer`:
  1. Page-size placeholders with correct aspect ratio from the page tree, so there is no layout shift.
  2. A cached thumbnail or low-DPI bitmap stretched to the current zoom.
  3. Full-res tiles (e.g. 256–512 px) for the viewport plus a margin, rendered in a worker or wasm with cancellation on scroll or zoom.
  4. During a pinch, apply a CSS transform only; re-render tiles on gesture end.
  Cap the total canvas memory and use pdf.js's mobile 5 MP limit as a reference point.
- (Opinion) Use optimistic UI for annotation creation, page rotate, reorder and delete: apply to the op log and the UI immediately, persist in the background, and roll back with a toast on failure. Show skeletons only for sidebars and search results, never a blank page.
- (Opinion) Budget: input acknowledgement under 16 ms (one frame) on pointer feedback, under 100 ms for visible results of discrete actions, and INP p75 at or under 200 ms tracked in CI with real-device traces.

### Gaps
- The RAIL model page (web.dev) could not be fetched to quote its 100 ms response / 16 ms animation figures. Those figures are from general knowledge and need a citation.

## Dark mode for PDFs (inversion vs smart dark mode) and theming tokens

### Takeaway
There are three tiers:
- Chrome-only dark: the UI goes dark and pages stay white. This is PDF Expert's default; system Dark Mode does not change the file.
- Theme recolouring: Night or Sepia themes, or pdf.js's `pageColors`/`forcePageColors` (defaults `Canvas`/`CanvasText`, so it follows forced-colours). PDFium has a forced-colours API that recolours text and paths only.
- Naive CSS `invert()`, which breaks images and charts unless images are re-inverted or hue-rotated.

### Cited Findings
- PDF Expert follows the system Dark Mode for its UI (iOS 13+, app v7.2.0+). On Mac the chosen mode doesn't affect the files themselves; Night and Sepia themes are offered for reading. — [PDF Expert help: Dark mode](https://pdfexpert.helpjuice.com/turn-on-the-dark-mode)
- pdf.js app options are `pageColorsBackground: "Canvas"`, `pageColorsForeground: "CanvasText"` and `forcePageColors: false`. The defaults are CSS system colours, which map to the user's high-contrast or forced-colours palette. — [pdf.js app_options.js](https://github.com/mozilla/pdf.js/blob/master/web/app_options.js)
- react-pdf 10.4.0 exposes a `pageColors` prop for overriding background and foreground colours, framed as an accessibility feature. — [react-pdf 10.4.0 release](https://newreleases.io/project/npm/react-pdf/release/10.4.0)
- PDFium added a public forced-colours API; the colour scheme is applied only to text and paths. — [PDFium commit: forced colors mode](https://pdfium.googlesource.com/pdfium.git/+/d35a62dfb59e581392e421848740250816993aab%5E%21/samples/pdfium_test.cc)
- Firefox's "allow pages to choose their own colors" preference interacts with the pdf.js viewer and is used by people with low vision to force high contrast. — [Mozilla bug 843473](https://bugzilla.mozilla.org/show_bug.cgi?id=843473)
- Naive inversion (e.g. `filter: invert(0.95)`) is common but turns images into negatives. Smart-dark extensions such as Night PDF use hue rotation to preserve the colours of charts, images and highlights. — [DEV: browser hacks for dark-mode PDFs](https://dev.to/adityads1806/browser-hacks-for-dark-mode-in-pdfs-4igp); [Chrome Web Store: Night PDF](https://chromewebstore.google.com/detail/lipmfnjbiiciimhjmpefebhdiimaamog)

### Inferences
- (Opinion) Offer four page-appearance modes, separate from the UI theme:
  1. Original (default, even in dark UI).
  2. Sepia.
  3. Dark: recolour at render time, mapping white backgrounds to the theme's dark surface and dark text or paths to light, leaving raster images untouched. This is how PDFium and pdf.js do it; it is better than a CSS invert.
  4. High contrast: follow `forced-colors`/`prefers-contrast`.
  Implementing tier 3 in the renderer (`lamina` core) keeps it identical on web, desktop and mobile.
- (Opinion) If CSS inversion is used as a stopgap, apply `invert(1) hue-rotate(180deg)` to the page and re-apply it to image layers so photos and colour-coded highlights keep their meaning. Annotation colours must stay true in every mode, because they are semantic.
- (Opinion) Use a three-layer token system:
  - Primitive (palette).
  - Semantic: `--surface`, `--surface-raised`, `--text`, `--accent`, `--border`, `--page-backdrop`, `--selection`.
  - Component.
  Ship light, dark and high-contrast sets, switched with `prefers-color-scheme` and a `data-theme` override, and keep the page backdrop (canvas gutter) a distinct token so dark UI doesn't produce "blinding grey margins".

### Gaps
- No official documentation was found on Microsoft Edge's built-in PDF dark-mode behaviour, or on Acrobat's 2023+ dark-theme or page-colour options.
- No user research comparing inverted vs recoloured dark PDF reading was found.
