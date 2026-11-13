# Paper: screen inventory

Every screen lamina needs, grouped by feature area, with the platforms it is designed for and the todos that build it. Each area is one Design canvas; the artboard file names match the **ID** column (for example `R01-home.dc.html`). The look follows the [Paper design language](design-language.md).

Platforms: **W** web app, **D** desktop app (Tauri; macOS frame drawn, Windows and Linux differ only in window controls and menu placement), **M** phone (390×844), **T** tablet (1024×1366). The web and desktop shells share one layout, so a screen marked **W/D** is drawn once (usually as the desktop window when the feature is desktop-first, else as the web app) and the other platform's differences are noted on the artboard.

Mode names follow the [UI/UX plan](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2): Read (default), Comment, Fill & Sign, Organise, Edit, plus Redact and Protect as tools.

## F. Foundations (the chosen Paper boards)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| F01 | Web reader v0.1 (Reader top bar), selection mini toolbar | W | [C4a](../../todos/done/C4a-ui-chrome-and-platform.md), [U1c](../../todos/done/U1c-inverted-l-shell-and-modes.md), [U2a](../../todos/done/U2a-selection-surfaces-and-copy-modes.md) |
| F02 | Desktop, Comment mode, inspector, command palette | D | [U1b](../../todos/done/U1b-command-registry-and-palette.md), [U1c](../../todos/done/U1c-inverted-l-shell-and-modes.md), [E4f](../../todos/ready/E4f-annotation-tools-ui.md) |
| F03 | Phone, reading | M | [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md) |
| F04 | Phone, text selected, Pages sheet | M | [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md), [U2a](../../todos/done/U2a-selection-surfaces-and-copy-modes.md) |
| F05 | Style tile | — | [U1a](../../todos/done/U1a-adr-0024-app-shell-ux.md), [C7a](../../todos/done/C7a-accessibility-and-theming.md) |

## R. Reader (web reader v0.1 and the shared viewer)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| R01 | Home / open: drop zone, open file, open from URL, recent files (local only), privacy line | W, M | [C4b](../../todos/done/C4b-web-app-shell.md), [C4c](../../todos/done/C4c-remote-files-encryption-privacy.md) |
| R02 | Loading states: placeholders at true aspect, low-res then sharp tiles, progress for large files | W | [C3b](../../todos/done/C3b-viewer-virtualiser-and-zoom.md) |
| R03 | Password prompt (password-required, wrong-password) | W, M | [C4c](../../todos/done/C4c-remote-files-encryption-privacy.md) |
| R04 | Repaired / damaged file: status-bar notice and details panel; unrecoverable error page | W | [C4b](../../todos/done/C4b-web-app-shell.md) |
| R05 | Find in document: find bar, match count, results list in the rail, case/whole-word options | W, M | [C5b](../../todos/done/C5b-find-in-document.md), [C5c](../../todos/done/C5c-find-results-tab.md) |
| R06 | Rail tabs: Pages (thumbnails with page labels) · Outline (nested, expand/collapse) · contextual tabs (Attachments, Results…) | W | [C6a](../../todos/done/C6a-thumbnails-outline-page-labels.md) |
| R07 | Links and history: link hover target preview, back/forward after following a link | W | [C6b](../../todos/done/C6b-links-and-history.md) |
| R08 | Document properties (dialog: title, author, PDF version, page size, fonts, security, tagged) | W | [C6c](../../todos/done/C6c-print-download-properties.md) |
| R09 | Print and Download a copy (print dialog: range, scale, page appearance warning) | W | [C6c](../../todos/done/C6c-print-download-properties.md) |
| R10 | View settings: UI theme (light, dark, high contrast, match system), accent colour (Teal, Blue, Plum, Green), page appearance (Original, Sepia, Dark, High contrast), page layout (single, two-up, continuous), zoom presets | W, M | [C7a](../../todos/done/C7a-accessibility-and-theming.md), [U2c](../../todos/done/U2c-page-appearance-modes.md) |
| R11 | Dark UI theme (F01 in dark chrome, page Original) | W | [C7a](../../todos/done/C7a-accessibility-and-theming.md) |
| R12 | Keyboard shortcuts (help generated from the command registry) | W | [U1b](../../todos/done/U1b-command-registry-and-palette.md), [C7a](../../todos/done/C7a-accessibility-and-theming.md) |
| R13 | Network activity panel: requests since open, per-origin list, "what this proves" | W | [U4a](../../todos/done/U4a-network-indicator-and-privacy-proof.md) |
| R14 | Reader mode (reflow): tagged (tier 1) with settings; approximate (tier 2) label; back to page | W, M | [U4b](../../todos/done/U4b-reader-mode-from-structure-tree.md), [U4c](../../todos/done/U4c-approximate-reader-mode.md) |
| R15 | Offline, install and update: install prompt, offline strip, update available | W | [C7b](../../todos/done/C7b-offline-pwa.md) |
| R16 | Focus mode (rails hidden; inspector returns on selection) | W | [U1c](../../todos/done/U1c-inverted-l-shell-and-modes.md) |
| R17 | Tablet reading: rail as sheet, pen hover | T | [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md) |
| R18 | Phone: outline, find, more (outline sheet, find, more menu, view settings sheet) | M | [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md) |

## S. Desktop shell

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| S01 | Start window: recent files with thumbnails, open, drop zone | D | [D3a](../../todos/done/D3a-open-from-os.md), [D3b](../../todos/done/D3b-recent-files-and-file-watching.md) |
| S02 | Window with document tabs and session restore: tab overflow, session restore notice | D | [D4a](../../todos/ready/D4a-windows-tabs-and-session-restore.md) |
| S03 | Native menus: macOS menu bar (File, Edit, View, Go, Tools, Window, Help) and Windows/Linux in-window menu | D | [D4b](../../todos/ready/D4b-native-menus.md) |
| S04 | File changed on disk: strip (Compare · Keep mine · Reload from disk); the next Save asks | D | [D3b](../../todos/done/D3b-recent-files-and-file-watching.md) |
| S05 | Print: native print dialog hand-off, printing progress | D | [D4c](../../todos/ready/D4c-native-printing.md) |
| S06 | Updates: update available / downloading / restart to update; channel note | D | [D5b](../../todos/ready/D5b-updater-in-app.md) |
| S07 | Settings window: General, Appearance, Privacy (telemetry opt-in, crash reports), Shortcuts (single-key on/off), Advanced (renderer switch) | D | [D7b](../../todos/ready/D7b-settings-renderer-switch-and-download-page.md), [D7a](../../todos/ready/D7a-crash-reporting.md), [U5c](../../todos/ready/U5c-opt-in-usage-telemetry.md) |
| S08 | Crash recovered: restore session and optional report preview | D | [D7a](../../todos/ready/D7a-crash-reporting.md) |
| S09 | Download page (web): per-OS download, checksums, system requirements | W | [D7b](../../todos/ready/D7b-settings-renderer-switch-and-download-page.md) |
| S10 | Windows window frame (title bar controls, menu bar) | D | [D4b](../../todos/ready/D4b-native-menus.md) |

## O. Edit session, organise, navigation and extraction (E1–E3)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| O01 | Edit session and history: unsaved state in title and status bar, undo/redo, History tab | D | [E1d](../../todos/done/E1d-edit-session-undo-ui.md) |
| O02 | Save, Save as and Download a copy: Save a copy, signed-file constraint banner, incremental vs full explanation | D, W | [E1e](../../todos/done/E1e-save-and-constraint-banner.md) |
| O03 | Close with unsaved changes (dialog) | D | [E2i](../../todos/ready/E2i-signed-gating-and-close.md) |
| O04 | Document info and XMP editor (title, author, subject, keywords, dates; XMP tree) | D | [E1f](../../todos/done/E1f-metadata-ui-capability-page.md), [E3e](../../todos/done/E3e-xmp-editor.md) |
| O05 | Capability page: what lamina can read, write and round-trip | W | [E1f](../../todos/done/E1f-metadata-ui-capability-page.md) |
| O06 | Organise mode: page grid, multi-select, insertion bar, rotate, delete, duplicate, "Move to page…" | D, W | [E2h](../../todos/ready/E2h-page-organiser-ui.md), [E2b](../../todos/ready/E2b-page-structure-ops.md) |
| O07 | Insert pages and merge documents (from another PDF, images, blank) | D | [E2c](../../todos/ready/E2c-cross-document-pages.md), [E2d](../../todos/ready/E2d-merge-and-split.md), [E3h](../../todos/done/E3h-images-to-pdf-and-page-export.md) |
| O08 | Split document (by count, ranges, bookmarks) | D | [E2d](../../todos/ready/E2d-merge-and-split.md) |
| O09 | Page geometry: crop and page boxes, resize, warning that cropped content remains | D | [E2f](../../todos/ready/E2f-page-geometry.md), [E2s](../../todos/ready/E2s-resize-and-orientation.md), [E2g](../../todos/ready/E2g-page-labels.md) |
| O10 | Page labels editor (roman front matter, numbering ranges) | D | [E2q](../../todos/ready/E2q-page-labels-op.md), [E2g](../../todos/ready/E2g-page-labels.md) |
| O11 | Bookmarks editor (nested, drag and keyboard reorder, from headings) | D | [E3b](../../todos/ready/E3b-bookmarks-and-destinations.md) |
| O12 | Link editor: link selected text or draw a link area; target page, named destination, web or email address; script address rejected | D | [E3c](../../todos/ready/E3c-links.md) |
| O13 | Initial view and article threads | D | [E3d](../../todos/ready/E3d-initial-view-and-threads.md) |
| O14 | Attachments panel (add, extract, associated files) | D | [E3f](../../todos/ready/E3f-attachments-and-associated-files.md) |
| O15 | Object inspector (object tree / COS inspector, advanced) | D | [E3g](../../todos/ready/E3g-inspectors.md) |
| O16 | Export and extraction: pages as images, text/Markdown/JSON, pictures out | D, W | [E3h](../../todos/done/E3h-images-to-pdf-and-page-export.md), [E3i](../../todos/ready/E3i-extraction.md) |
| O17 | Phone: organise pages (grid, select, reorder by "Move to"), bottom action bar | M | [E2h](../../todos/ready/E2h-page-organiser-ui.md) |

## C. Comment and review (E4–E5)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| C01 | Comment tools: highlight, underline, strike, caret, note, ink, shapes, arrow, text box; colour and width | D | [E4f](../../todos/ready/E4f-annotation-tools-ui.md) |
| C02 | Ink and shapes: drawing in progress, selected shape with handles, inspector | D | [E4d](../../todos/ready/E4d-appearances-ink-and-shapes.md) |
| C03 | Free text and callouts: editing | D | [E4e](../../todos/ready/E4e-freetext-callouts.md) |
| C04 | Comments panel: threads, replies, status (accepted, rejected, done), filter and search | D, W | [E5b](../../todos/ready/E5b-comment-threads-and-review.md), [E5g](../../todos/ready/E5g-comments-panel.md) |
| C05 | Stamps and file attachments (stamps picker: standard, custom image, dynamic date/name; file attachment annotation) | D | [E5c](../../todos/ready/E5c-stamps-and-attachments.md), [E5h](../../todos/ready/E5h-stamps-and-attachments-ui.md) |
| C06 | Import and export comments (XFDF/FDF), export highlights as Markdown | D | [E5d](../../todos/ready/E5d-xfdf-fdf.md), [E5i](../../todos/ready/E5i-interchange-flatten-summary-ui.md) |
| C07 | Flatten and comment summary (print/export) | D | [E5e](../../todos/ready/E5e-flatten-and-summary.md), [E5f](../../todos/ready/E5f-comment-summary.md), [E5i](../../todos/ready/E5i-interchange-flatten-summary-ui.md) |
| C08 | Phone annotate (pen, highlighter, eraser), comment sheet | M | [E4f](../../todos/ready/E4f-annotation-tools-ui.md), [U3a](../../todos/done/U3a-responsive-mobile-and-touch.md) |
| C09 | Tablet stylus: ink with palm rejection, tool shelf | T | [E4f](../../todos/ready/E4f-annotation-tools-ui.md) |

## A. Add and edit content (E6–E7)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| A01 | Edit mode: add text box, font controls in inspector | D | [E6c](../../todos/ready/E6c-add-text.md), [E6e](../../todos/ready/E6e-content-tools-ui.md) |
| A02 | Place image and shapes (as content, not annotations), transform handles, opacity | D | [E6b](../../todos/ready/E6b-place-images-and-shapes.md) |
| A03 | Watermark, header and footer dialog with live preview and page ranges | D | [E6d](../../todos/ready/E6d-watermarks-headers-footers.md) |
| A04 | Edit existing text inline: text run selection, font substitution warning | D | [E7e](../../todos/ready/E7e-text-runs-and-inline-edit.md), [E7f](../../todos/ready/E7f-font-handling-and-styling.md) |
| A05 | Select and transform existing objects (images, paths), arrange, delete | D | [E7d](../../todos/ready/E7d-select-transform-existing.md), [E7h](../../todos/ready/E7h-vector-clipping-masks.md) |
| A06 | Search and replace across the document | D | [E7g](../../todos/ready/E7g-search-and-replace.md) |
| A07 | Structure and conformance warning when editing tagged or PDF/A files | D | [E7i](../../todos/ready/E7i-structure-and-conformance-maintenance.md) |

## P. Forms (E8–E9)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| P01 | Fill & Sign mode: fields highlighted, text, checkbox, radio, dropdown, date; required marker; tab order | W, D | [E8e](../../todos/ready/E8e-field-widgets-ui.md) |
| P02 | Validation, formatting and calculated fields; read-only fields | W | [E8f](../../todos/ready/E8f-declarative-behaviour.md) |
| P03 | Form data: import/export (FDF, XFDF, JSON), reset, flatten | D | [E8g](../../todos/ready/E8g-form-data-and-flatten.md) |
| P04 | Phone: fill a form field by field (keyboard, next/previous) | M | [E8e](../../todos/ready/E8e-field-widgets-ui.md) |
| P05 | Form design mode: field palette, place and resize, field properties inspector | D | [E9c](../../todos/ready/E9c-form-design-mode.md), [E9b](../../todos/ready/E9b-field-creation-ops.md) |
| P06 | Tab order and accessible labels (editor) | D | [E9c](../../todos/ready/E9c-form-design-mode.md) |
| P07 | Automatic field detection review (accept / reject suggestions) | D | [E9d](../../todos/ready/E9d-automatic-field-detection.md) |

## G. Sign, protect and redact (E10–E12)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| G01 | Add a signature image: draw, type, upload; placement on the page | W, D, M | [E10c](../../todos/ready/E10c-visual-signatures.md), [E10j](../../todos/ready/E10j-signature-ui.md) |
| G02 | Sign with a certificate: choose identity (file, platform keychain, token), reason, appearance, timestamp | D | [E10j](../../todos/ready/E10j-signature-ui.md), [E10i](../../todos/ready/E10i-platform-signers.md), [E10e](../../todos/ready/E10e-timestamps-and-tsa-proxy.md) |
| G03 | Certify and lock (DocMDP level, field locks) | D | [E10f](../../todos/ready/E10f-certification-and-locking.md) |
| G04 | Signature validation: badge on open, signatures panel (valid, unknown signer, modified after signing) | W, D | [E10g](../../todos/ready/E10g-signature-validation.md) |
| G05 | Revision diff and attack warnings (compare view on the signed revision) | D | [E10h](../../todos/ready/E10h-revision-diff-and-attacks.md) |
| G06 | Protect: encrypt with password, permissions, AES-256; remove security; certificate (public-key) security | D | [E11b](../../todos/ready/E11b-encrypt-on-write.md), [E11c](../../todos/ready/E11c-remove-security.md), [E11e](../../todos/ready/E11e-public-key-security.md) |
| G07 | Sanitise: what will be removed (metadata, JavaScript, attachments, hidden layers) and the report | D | [E11d](../../todos/ready/E11d-sanitiser.md) |
| G08 | Redact mode: mark and review (mark text and areas, review list) | D | [E12b](../../todos/ready/E12b-mark-and-review.md) |
| G09 | Find sensitive content (patterns: emails, card numbers, regex) to mark | D | [E12e](../../todos/ready/E12e-sensitive-search.md) |
| G10 | Apply redactions and verification report: confirmation; save blocked while leaks remain | D | [E12c](../../todos/ready/E12c-apply-content-removal.md), [E12d](../../todos/ready/E12d-scrub-and-rewrite.md) |
| G11 | Phone: sign a document (signature sheet, place, save) | M | [E10j](../../todos/ready/E10j-signature-ui.md) |

## X. Convert, compare, batch, accessibility, print production, advanced (E13–E17)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| X01 | Make searchable (OCR): language, local vs server job, progress, result | D | [E13c](../../todos/ready/E13c-ocr-text-layer.md), [E13d](../../todos/ready/E13d-local-ocr.md) |
| X02 | Convert to and from PDF (Office, HTML to PDF; PDF to Word, Excel): job sheet with server-processing consent | W, D | [E13f](../../todos/ready/E13f-office-and-html-to-pdf.md), [E13g](../../todos/ready/E13g-pdf-to-word-excel.md), [E13e](../../todos/ready/E13e-server-jobs-infrastructure.md) |
| X03 | Revision browser: incremental updates timeline, open a revision read-only | D | [E14b](../../todos/ready/E14b-revision-browser.md) |
| X04 | Compare two versions: side by side, overlay/pixel diff, text and annotation changes list | D | [E14c](../../todos/ready/E14c-compare.md) |
| X05 | Batch runner: saved action list, files queue, results | D | [E14d](../../todos/ready/E14d-batch-runner.md), [E14e](../../todos/ready/E14e-server-batch.md) |
| X06 | Accessibility checker: issues list with fix actions | D | [E15d](../../todos/ready/E15d-accessibility-checks.md) |
| X07 | Tags and reading order (structure tree, reading-order overlay), alt text | D | [E15b](../../todos/ready/E15b-structure-tree-editing.md), [E15c](../../todos/ready/E15c-tagging-tools.md) |
| X08 | Save as PDF/UA or PDF/A with conformance result | D | [E15e](../../todos/ready/E15e-pdfua-output.md), [E16e](../../todos/ready/E16e-preflight-and-conversion.md) |
| X09 | Optimise: size estimate, images, fonts, unused objects, linearise | D | [E16b](../../todos/ready/E16b-structural-optimiser.md), [E16c](../../todos/ready/E16c-images-and-fonts.md) |
| X10 | Print production: colour and output intents, preflight report, imposition | D | [E16d](../../todos/ready/E16d-colour.md), [E16e](../../todos/ready/E16e-preflight-and-conversion.md), [E16f](../../todos/ready/E16f-imposition.md) |
| X11 | Layers and actions (optional content panel, actions editor) | D | [E17c](../../todos/ready/E17c-layers.md), [E17b](../../todos/ready/E17b-action-editor.md) |
| X12 | Measurement and media (distance, area, scale; media and 3D placeholders) | D | [E17e](../../todos/ready/E17e-measurement.md), [E17d](../../todos/ready/E17d-media.md), [E17f](../../todos/ready/E17f-3d-detection.md) |

## H. Account, cloud, collaboration and product pages (H, E18, U5, W1)

| ID | Screen | Platforms | Todos |
| --- | --- | --- | --- |
| H01 | Sign in / create account (optional; local use needs none) | W, M | [H2](../../todos/ready/H2-accounts-and-login.md) |
| H02 | Account and plan: [PLAN NAME], [PRICE], licence key for offline use, devices | W | [H2](../../todos/ready/H2-accounts-and-login.md), [U5a](../../todos/ready/U5a-pricing-and-promises-page.md) |
| H03 | Cloud documents and upload consent: list with sync status | W, M | [H3](../../todos/ready/H3-cloud-document-storage.md), [H4](../../todos/ready/H4-document-sync.md) |
| H04 | Sync conflict and offline edits | W | [H4](../../todos/ready/H4-document-sync.md) |
| H05 | Share for review (dialog: people, link, permissions) | W | [E18f](../../todos/ready/E18f-sharing-and-notifications.md) |
| H06 | Shared review: presence, live comments, lease on content edits | W | [E18e](../../todos/ready/E18e-client-sync-and-presence.md), [E18g](../../todos/ready/E18g-leases-and-base-revisions.md) |
| H07 | Notifications (in-app and email preview) | W | [E18f](../../todos/ready/E18f-sharing-and-notifications.md) |
| H08 | Pricing and promises page | W | [U5a](../../todos/ready/U5a-pricing-and-promises-page.md) |
| H09 | Benchmark page | W | [U5b](../../todos/ready/U5b-reader-benchmark.md) |
| H10 | Telemetry opt-in and what we collect | W, D | [U5c](../../todos/ready/U5c-opt-in-usage-telemetry.md) |
| H11 | Upgrade prompt before work starts (never after) | W | [U5a](../../todos/ready/U5a-pricing-and-promises-page.md) |
| H12 | Website home, for now the roadmap: what Lamina PDF is, a first PDF in code, the milestones, the dashboard, the packages | W, M | [W1c](../../todos/done/W1c-home-page.md) |
| H13 | How we test: the latest full night's figures with what each counts, the licence policy, where the code comes from | W, M | [T1b](../../todos/done/T1b-how-we-test-page.md) |

## Canvases

Each canvas is private to the owner until shared. Its source files are committed in the folder listed (`project/canvas.json` plus one `.dc.html` per artboard); the files render inside the Design canvas only. When a design changes, update the files here and republish the canvas from them.

| Screens | Area | Document | Canvas | Files |
| --- | --- | --- | --- | --- |
| F | Paper foundations | this page | [canvas](https://claude.ai/artifact/EdzB5wLs6neApuNbwN6Jmr) | [`foundations/`](foundations/project/canvas.json) |
| R01–R09 | Reader A | [reader-a.md](reader-a.md) | [canvas](https://claude.ai/artifact/MWpEwMqZSWUBYSMuCi1Fz2) | [`reader-a/`](reader-a/project/canvas.json) |
| R10–R18 | Reader B | [reader-b.md](reader-b.md) | [canvas](https://claude.ai/artifact/KaCtzmB4hNEHgyJJQv2HBi) | [`reader-b/`](reader-b/project/canvas.json) |
| S | Desktop shell | [desktop-shell.md](desktop-shell.md) | [canvas](https://claude.ai/artifact/RMvUvGA8RAG3AHmdCZYnfw) | [`desktop-shell/`](desktop-shell/project/canvas.json) |
| O01–O08 | Edit session and organise | [edit-session-organise.md](edit-session-organise.md) | [canvas](https://claude.ai/artifact/R2ZvF977LNg99a85zQiEeH) | [`edit-session-organise/`](edit-session-organise/project/canvas.json) |
| O09–O17 | Pages, navigation and extraction | [pages-navigation-extraction.md](pages-navigation-extraction.md) | [canvas](https://claude.ai/artifact/XxRzeJ46jd6mBFhp7HevYf) | [`pages-navigation-extraction/`](pages-navigation-extraction/project/canvas.json) |
| C | Comment and review | [comment-review.md](comment-review.md) | [canvas](https://claude.ai/artifact/QCantcmCvDnr79QABZAHUo) | [`comment-review/`](comment-review/project/canvas.json) |
| A | Add and edit content | [add-edit-content.md](add-edit-content.md) | [canvas](https://claude.ai/artifact/5CecE9xXiw2RXoVde7xEsL) | [`add-edit-content/`](add-edit-content/project/canvas.json) |
| P | Forms | [forms.md](forms.md) | [canvas](https://claude.ai/artifact/Jat39ahU8QNgrb7TLi3mZR) | [`forms/`](forms/project/canvas.json) |
| G | Sign, protect and redact | [sign-protect-redact.md](sign-protect-redact.md) | [canvas](https://claude.ai/artifact/QbwnMXXDikC4Fe8gwede5P) | [`sign-protect-redact/`](sign-protect-redact/project/canvas.json) |
| X | Convert, compare and produce | [convert-compare-produce.md](convert-compare-produce.md) | [canvas](https://claude.ai/artifact/GwyjtEGo2D2wMo4VUubKKZ) | [`convert-compare-produce/`](convert-compare-produce/project/canvas.json) |
| H | Account, cloud and product | [account-cloud-product.md](account-cloud-product.md) | [canvas](https://claude.ai/artifact/Ronx2eZUMeMbgZGbx8CYhX) | [`account-cloud-product/`](account-cloud-product/project/canvas.json) |
| H12 | Website home | [account-cloud-product.md](account-cloud-product.md#h12-website-home) | [canvas](https://claude.ai/artifact/2doFGoyD8t9KJZZnLAwGAQ) | [`website-home/`](website-home/project/canvas.json) |
| H13 | How we test | [account-cloud-product.md](account-cloud-product.md#h13-how-we-test) | not published yet: the files are drawn from the built page | [`website-home/`](website-home/project/canvas.json) |
