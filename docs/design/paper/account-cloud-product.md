# Paper: account, cloud, collaboration and product pages

- **Status:** Draft (Paper direction, 2026-10-29)
- **Canvas:** [lamina Paper — Account, cloud and product](https://claude.ai/artifact/Ronx2eZUMeMbgZGbx8CYhX)
- **Inventory:** [screens.md, section H](screens.md#h-account-cloud-collaboration-and-product-pages-h-e18-u5) · **Look and components:** [design-language.md](design-language.md)
- **Plans:** [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and [optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features), [roadmap: decisions made](../../plans/roadmap.md#decisions-made), [UI/UX principle 10](../../plans/ui-ux.md#4-principles) and [§5.9](../../plans/ui-ux.md#59-product-promises-r18-r19)

This area covers everything that involves our server or our prices: the optional account (H01, H02), cloud documents and sync (H03, H04), shared review (H05–H07), the product pages on the docs site (H08, H09), opt-in usage counts (H10) and the upgrade prompt (H11).

Rules every screen here follows:

- **Local use never needs an account.** Sign-in is offered, never required; nothing in the reader, editor or save path checks an account ([roadmap: hosted service](../../plans/roadmap.md#decisions-made), H2 acceptance criteria).
- **Nothing leaves the device without a visible, per-action choice.** Every request our server receives is counted by the network indicator, which names the destination ("to lamina’s storage only").
- **Priced, never hostile** ([principle 10](../../plans/ui-ux.md#4-principles)): the plan and price show before work starts; no watermarks; no changed deals. Pricing is not decided, so every price is `[PLAN NAME]`, `[PRICE]`, `[BILLING TERMS]`, `[QUOTA]`; the owner fills them in [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).
- Other undecided values are bracketed too: `[REGION]` (data residency), `[N]` (days to delete an account), `[RETENTION]` (usage counts), `[LAMINA DOMAIN]`, `[VERSION]`, `[SPEC]`, and every benchmark number (`[n ms]`, `[n MB]`).

Artboards live on the canvas; web screens are 1440-wide PAGE boards, phone boards are 390 × 844.

## H01 Sign in / create account

**Purpose.** Let someone who wants cloud storage, sync or shared review sign in, without suggesting that anything else needs it.

**Entry points.** More menu › "Sign in…" (`account.sign-in`); the "Sign in" text button on the Files home top bar (signed out); palette "Sign in"; the H05 share dialog and the H03 Cloud tab when signed out ("Sign in to use the cloud"). No shortcut.

**Layout.** Web: a dialog (design language [§7.18](design-language.md#718-dialog-proposed)) over the dimmed reader; the reader keeps its top bar, page and status bar. Phone: the same content in a bottom sheet at the large detent over the Files screen, actions full width.

**Content and controls.**

- Title "Sign in to lamina"; close icon button ("Close sign in").
- One sentence that local use needs no account and what an account adds.
- Email field (label "Email", `autocomplete="email"`), help text "We’ll email you a one-time sign-in code. Your open document stays on this device."
- "Not now" (secondary, closes) and "Continue" (primary, `account.request-code`).
- Footer: "Create an account" link (`account.create`, same flow with a name field), "What an account stores" (privacy page).

**States.**

- Email entry (drawn).
- Code sent: the field becomes "Sign-in code" (6 digits, `autocomplete="one-time-code"`, numeric keypad on touch), with "Send a new code" ghost and the address shown above it.
- Wrong or expired code: error text under the field in `danger` ("That code has expired. Send a new one."), `aria-invalid`.
- Offline: the dialog explains "Signing in needs a connection. Everything on this device keeps working." and "Continue" is disabled with that reason.
- Signed in: the dialog closes, a toast says "Signed in as r.osei@northwind.example"; the top bar of the Files home shows the account avatar.
- The sign-in method (email code, password, passkey, providers) depends on the [H1 auth spike](../../todos/ready/H1-auth-spike.md); the drawing assumes an email code, the layout takes extra provider buttons above the email field.

**Behaviour.** Sign-in never touches the open file and never reloads the page. Requests go to our origin only (`/api/auth/*`) and are counted by the network indicator. Escape or "Not now" closes and returns focus to the opener.

**Accessibility.** Focus starts in the email field (ring drawn). Dialog `role="dialog"`, `aria-modal`, labelled by the title and described by the intro sentence. Errors are linked with `aria-describedby`. Phone inputs are 16 px and 44 px high.

**Platform differences.** Web: in-app dialog, cookie session. Desktop: "Continue" opens the system browser (H2: deep-link callback, token in the OS keychain); the dialog shows "Finish signing in in your browser" with "Open the browser again" and "Cancel" until the callback arrives. Phone: sheet as drawn. Tablet: the web dialog with touch sizes.

**Artboards.** `H01-sign-in.dc.html`, `H01-sign-in-phone.dc.html`.

**Todos.** [H2](../../todos/ready/H2-accounts-and-login.md).

## H02 Account and plan

**Purpose.** One place for the account: who is signed in, the plan and its price, the licence key that lets on-device features work offline, signed-in devices, data export and deletion.

**Entry points.** Account avatar › "Account and plan" (`account.open`); Settings › Account and plan; palette "Account and plan". No shortcut.

**Layout.** Settings page: top bar with Back ("Back to Q3-board-report.pdf") and "Settings"; a 220 px settings nav on the left (General, View and themes, Shortcuts, Privacy, Account and plan, Notifications; current item `accent-tint`, `aria-current="page"`); content up to 960 px wide in a two-column grid that becomes one column below ~900 px; the status bar keeps the network indicator.

**Content and controls.**

- Header: avatar, name (page heading), email and "signed in since"; "Edit name or email" (secondary), "Sign out" (ghost, `account.sign-out`).
- **Plan:** `[PLAN NAME]` with an "Active" badge, `[PRICE] · [BILLING TERMS] · bought <date>`, what it includes, and the promise "If our plans change, what you paid for keeps working on the terms you bought it on." Actions: "Change plan" (`plan.change`), "Receipts" (`plan.receipts`), "Pricing and promises" (H08).
- **Licence key for offline use:** read-only masked key field, "Show" (toggles the mask), "Copy key" (`account.copy-licence-key`), "Download licence file" (`account.download-licence`). Copy: "Features that run on your device check this key on the device, offline. They never need you to sign in or reach our server."
- **Signed-in devices:** one row per device (icon, name, app, last active); the current one has a "This device" badge; others have "Sign out" (`account.sign-out-device`). Note: "Signing a device out stops its sync. Files saved on it stay there."
- **Your data:** cloud storage used (progress bar, `38.2 MB of [QUOTA]`), "Export my data" (`account.export-data`), "Delete account…" (`account.delete`, danger ghost).

**States.**

- No plan: the Plan block shows "No plan" and "See plans" instead of the badge; the licence block explains how to add a key bought elsewhere ("Enter a licence key…").
- Payment failed or plan ended (if the model has renewals): a `warning` notice in the Plan block with the date on which hosted features stop; on-device features bought outright keep working.
- Key copied: toast "Licence key copied".
- Delete account: an `alertdialog` that names what is deleted (account, cloud documents, shares) and what is not (files on devices), with a typed confirmation and the danger-filled "Delete account".
- Export: progress in place, then a download.

**Behaviour.** Nothing here changes a document. Signing out a device revokes its session; its next sync attempt asks it to sign in again. Plan changes go to the payment flow (outside lamina’s UI; to be designed with the payment todo U5a will add).

**Accessibility.** Each block is a `section` with a heading; the key field has a real label; "Show" announces its state ("Show licence key", `aria-pressed`). One focus ring drawn (Copy key).

**Platform differences.** Web: as drawn. Desktop: the same pane inside the Settings window; "Download licence file" becomes "Save licence file…" (system save dialog), and a desktop that is signed out can still "Enter a licence key…". Phone: the settings nav becomes a list screen; each block is its own screen.

**Artboards.** `H02-account.dc.html`.

**Todos.** [H2](../../todos/ready/H2-accounts-and-login.md), [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).

## H03 Cloud documents and upload consent

**Purpose.** Show the documents a signed-in user chose to upload, with each one’s sync status, and make every upload an explicit, informed choice.

**Entry points.** Files home › Cloud tab (`cloud.open-list`); "Upload a file…" on that tab (`cloud.upload`); More › "Save to cloud…" in the reader (`cloud.upload-current`); the share dialog for a local file ("Upload to share"). No shortcuts.

**Layout.** Web: Files home with a top bar (title "Files", Commands, account avatar), segmented tabs "On this device · Cloud · Shared with me", a header row (page heading, one-line explanation, storage meter, primary "Upload a file…") and a table. Upload consent: a dialog over the reader. Phone: the same tabs (44 px), storage line, a list of 72 px rows and a docked "Upload a file…" button above the home indicator.

**Content and controls.**

- Table columns: Name (opens the document, `cloud.open`), Sync, Size, Last saved (time · device), and a More button per row (Open, Download, Rename, Share…, Remove from cloud: `cloud.download`, `cloud.rename`, `share.open`, `cloud.delete`).
- Sync statuses, each a badge with an icon and a word (never colour alone): **Synced** (positive), **Uploading… 62%** (progress bar, "Cancel upload"), **Newer on MacBook Air** (neutral, sync icon: another device saved a newer revision; opening downloads it), **Conflict copy** (warning, see H04), **Waiting to upload** (neutral, offline), **Shared with 3** (accent, added beside the sync badge).
- Footnote: "The web app syncs a document when you open or save it. The desktop app also syncs in the background."
- Upload consent dialog: title "Upload Q3-board-report.pdf to your cloud?", "Nothing has been uploaded yet.", and a definition list: **What is sent** (the saved file, size and pages, with its comments and form values), **Where it goes** (`[REGION]`, only you until you share), **After this** (each save uploads a new version; the local copy stays), **What we do with it** (stored for your own use only; delete any time). Actions: "Keep on this device" (secondary), "Upload" (primary). "Privacy page" link.

**States.**

- Signed out: the Cloud tab shows an empty state, "Your cloud documents appear here" / "Sign in to upload documents you choose. Nothing is uploaded otherwise." with "Sign in".
- Empty (signed in): "No cloud documents yet" / "Upload a file, or choose Save to cloud from a document’s More menu."
- Unsaved changes when uploading: the consent dialog says "Your 1 unsaved change isn’t included. Save first to include it." and offers "Save and upload".
- Quota nearly full (≥ 90%): the meter turns `warning` and a notice above the table names what is using the space; an upload that doesn’t fit is refused before it starts, with the size and the space left.
- Upload failed: row badge "Upload failed" (danger) with "Retry".

**Behaviour.** Nothing uploads without the consent dialog the first time a document goes to the cloud; afterwards each save of that document uploads a new revision (H3: incremental saves dedupe by hash). "Remove from cloud" deletes the cloud copy and its revisions after an `alertdialog`; the local file is not touched. The network indicator counts uploads and names our storage as the only destination.

**Accessibility.** A real `<table>` with column headers; per-row More buttons are labelled with the file name; progress bars have `role="progressbar"` and values; tabs use `role="tablist"`. The consent dialog’s initial focus is on "Keep on this device" in production (the drawing rings "Upload" to show the ring on the primary).

**Platform differences.** Web: syncs on open and save (OPFS cache). Desktop: the Cloud tab adds "Sync now" and shows background progress; the consent dialog adds "Keep the file on this computer as the original" (on by default; H3: local files stay the source of truth unless the user opens the cloud copy). Phone: drawn; row actions open a bottom sheet. Tablet: the web table with 44 px rows.

**Artboards.** `H03-cloud-documents.dc.html`, `H03-upload-consent.dc.html`, `H03-cloud-documents-phone.dc.html`.

**Todos.** [H3](../../todos/ready/H3-cloud-document-storage.md), [H4](../../todos/ready/H4-document-sync.md).

## H04 Sync conflict and offline edits

**Purpose.** When two devices save from the same revision, keep both and say so plainly; when offline, keep saving locally and show what will upload later. Whole-file revisions only; live co-review is H06 (H4 scope).

**Entry points.** The conflict dialog appears after a save is rejected because its base revision is stale. The status-bar item "Conflict copy · Details" and the H03 "Conflict copy" badge reopen it (`sync.show-conflict`). The offline item "2 saves waiting to upload" opens the queue popover (`sync.show-queue`).

**Layout.** Conflict: dialog (560 px) over the reader, which now shows the conflict copy as the open file. Offline: a `surface` notice strip under the secondary toolbar ("You’re offline. …") and a popover anchored to the status-bar item.

**Content and controls.**

- Conflict dialog: title "Q3-board-report.pdf changed on another device"; one sentence that both versions were kept; two side-by-side blocks, "Your version · this browser" (new name `Q3-board-report (conflict, this browser, 29 Oct 2026).pdf`, saved time, a summary of changes, "Open now" badge) and "MacBook Air’s version" (keeps the original name, saved time, changes); a hint on how to end up with one file. Actions: "Compare the two" (`compare.open`, from the compare round), "Open MacBook Air’s version" (`sync.open-other`), "Keep working on my copy" (primary, closes).
- Offline notice: cloud-off icon, "**You’re offline.** Saves stay on this device and upload when you’re back online. Reading, commenting and saving all keep working."
- Status bar: network indicator "0 network requests · offline"; sync item "2 saves waiting to upload".
- Queue popover: "Waiting to upload", one row per queued save (time, change summary), the rule that a later conflict keeps both, "Download a copy" (secondary) and "Upload now" (disabled while offline, reason under it).

**States.**

- Conflict (drawn); conflict resolved (the user deletes one copy from the cloud; the status item disappears).
- Offline with a queue (drawn); reconnecting: the item reads "Uploading 1 of 2…"; done: toast "2 saves uploaded"; upload failed: item in `danger` "1 save didn’t upload · Retry".
- Change summaries come from the op log where available; if not, the block shows only the saved time.

**Behaviour.** No data is ever dropped: the server rejects the stale base and the client stores its save as a conflict copy (H4). The file on disk is not modified by sync except by downloading a newer revision when the user opens it. Undo is unaffected (sync is not an edit).

**Accessibility.** The conflict dialog is described by its first sentence; the two version blocks are plain text, not colour-coded. The offline notice is `role="status"` so screen readers hear it once. The queue popover is a non-modal dialog; Escape closes it and returns focus to the status item.

**Platform differences.** Web: offline saves go to the OPFS cache and upload while the document is open or the next time it is opened. Desktop: a background queue uploads as soon as the connection returns, even with the document closed; the conflict dialog can also appear as a system notification. Phone: the dialog becomes a large-detent sheet; the offline notice sits under the top bar.

**Artboards.** `H04-sync-conflict.dc.html`, `H04-offline-edits.dc.html`.

**Todos.** [H4](../../todos/ready/H4-document-sync.md).

## H05 Share for review

**Purpose.** Invite people to review a cloud document, or turn on a review link, with a role each, and take access away.

**Entry points.** "Share" button in the top bar (shown when signed in), More › "Share for review…" (`share.open`), H03 row More › "Share…". No shortcut.

**Layout.** Dialog (560 px) over the reader in Comment mode.

**Content and controls.**

- Title "Share for review"; intro: who can open what, and that comments stay beside the PDF until saved into it (base PDFs are immutable; ADR 0012 via [E18f](../../todos/ready/E18f-sharing-and-notifications.md)).
- "Add people by email" field, a role select (Can comment by default; Can view; Can edit), "Invite" (primary, `share.invite`).
- People list: owner (no controls), each member with a role select (`share.change-role`) and a remove button ("Remove A. Patel", `share.remove-person`); pending invites say "Invited 28 Oct · not opened yet".
- Role help: "Can view: read and see comments. Can comment: also add comments and replies. Can edit: also change form values, page order and text, one person at a time for text."
- Review link: switch (`share.toggle-link`), read-only link field, link role select, "Copy link" (`share.copy-link`), and "Turning the link off, or removing a person, ends their access at once, even if they have the document open."
- "Email me about new comments and replies" checkbox (off by default; `notifications.toggle-email`), "Done".

**States.**

- Local file: the dialog first shows the upload consent content from H03 with the primary "Upload and share".
- Signed out: "Sign in to share" with the H01 flow.
- No one invited: the people list shows only the owner and "No one else has access".
- Invite sent: toast "Invited t.okafor@northwind.example"; invalid email: field error.
- Link off: field and "Copy link" disabled; link copied: toast "Link copied".
- Whether a link needs the visitor to sign in is ADR 0012’s Q6; the link help text changes to match ("People with the link sign in first" or not).

**Behaviour.** Removing a person or turning off the link calls the revoke path: open sessions are closed at once (E18f, close code 4403) and the removed person sees "Your access was removed" (H06 state). Roles are enforced on the server.

**Accessibility.** Every select has a visually hidden label naming the person; the switch is `role="switch"` labelled "Review link"; initial focus in the email field (ring drawn); the list is a `ul` labelled "People with access".

**Platform differences.** Identical on web and desktop (E18f). Phone: a full-height sheet; per-person roles open a small sheet instead of a select.

**Artboards.** `H05-share.dc.html`.

**Todos.** [E18f](../../todos/ready/E18f-sharing-and-notifications.md).

## H06 Shared review

**Purpose.** Review a shared document together: see who is here and what they have selected, get comments and replies live, and keep content edits from colliding (leases).

**Entry points.** Opening a shared cloud document (from H03, H07 or a review link). The presence button lists who is here (`review.show-people`).

**Layout.** The normal shell. Top bar: save-state text "Shared · comments sync live", a presence stack (other people’s initials in their presence colour, "+ you") and "Share". Secondary toolbar for the current mode. Lease notice strip under it when needed. Inspector with the selected thread. Status bar: network indicator naming the review server, a sync item ("Live · comments synced 10:32"), page label.

**Content and controls.**

- **Presence:** each reviewer gets a colour from a fixed set with white text ≥ 4.5:1 (drawn: `positive` #2F6A45 for A. Patel, `warning` #8A5300 for J. Mensah). Their current selection is outlined on the page in that colour with a name tag ("J. Mensah"). Your own selection keeps the `accent` focus outline.
- **Live thread:** the inspector shows the quoted text, messages with author and time, and a live "J. Mensah is replying…" row (`aria-live="polite"`); reply field, "Reply" (primary, `comment.reply`) and "Resolve" (`comment.resolve`).
- **Lease** ([E18g](../../todos/ready/E18g-leases-and-base-revisions.md)): drawn in Edit mode while A. Patel holds the text lease. Text, Image and Redact tools are `aria-disabled` with the context note "Text, image and redaction tools wait for A. Patel"; the notice reads "**A. Patel is editing text on page 7** (since 10:20). Text edits take turns so nobody’s work is overwritten. Comments, links and form values still sync live." with "Tell me when it’s free" (`review.notify-lease-free`).

**States.**

- Live (drawn); reconnecting: the sync item reads "Reconnecting…" and new comments queue locally; offline (desktop): "Offline · 3 comments waiting", sent on reconnect (E18e).
- You hold the lease: notice "You’re editing text. Others can comment meanwhile." with "Finish editing" (`review.release-lease`); saving mints a new base revision and other clients rebase.
- Lease freed: toast "A. Patel finished editing. Text tools are available."
- Redaction applied in a shared document: notice that earlier versions were purged from lamina’s storage and caches, and that copies saved outside lamina are out of reach (E18g).
- Access removed: a dialog "Your access was removed" with "Download your comments" (if allowed) and "Close"; unsent changes are discarded and the dialog says how many.

**Behaviour.** Comments, highlights, form values and page order sync as records (LWW, deletes win); undo of your own change works after remote changes (ADR 0006 §4). PDF bytes never cross the live connection. Content edits, redaction and signing need the lease or run as a server job.

**Accessibility.** The presence button’s label lists everyone ("3 people here: you, A. Patel, J. Mensah"). Presence colours always come with a name tag. New remote comments are announced politely, at most one announcement per thread per few seconds. Disabled tools explain why via the context note. Focus ring drawn on the reply field.

**Platform differences.** Web and desktop identical; desktop also works offline and syncs on reconnect. Phone: presence collapses to a count in the top bar subtitle; the thread is a medium-detent sheet; the lease notice sits under the top bar.

**Artboards.** `H06-shared-review.dc.html`.

**Todos.** [E18e](../../todos/ready/E18e-client-sync-and-presence.md), [E18g](../../todos/ready/E18g-leases-and-base-revisions.md).

## H07 Notifications

**Purpose.** Tell people about replies, mentions, shares and removed access, in the app and, if they opt in, by email.

**Entry points.** The bell icon button in the top bar, shown when signed in ("Notifications, 2 unread"; `notifications.open`); Settings › Notifications. No shortcut.

**Layout.** In-app: a popover (menu shadow, `surface`, 400 px) under the bell. Email: a plain one-column message, 560 px content width.

**Content and controls.**

- Popover: title "Notifications", "Mark all as read" (`notifications.mark-all-read`); "Today" / "Earlier" overlines; rows as links (avatar, "**J. Mensah** replied on Q3-board-report.pdf", the comment text, "Page 3 · 10:34", and a "New" marker with a dot for unread). Selecting a row opens the document at the thread and marks it read. Footer: "Email notifications are off" + "Notification settings".
- Email: subject "J. Mensah replied on Q3-board-report.pdf"; heading, context sentence, the reply text, "Open the comment" button-link, a line that the email has the reply text and the document’s name but never the document or its pages, and "Turn off emails for this document" · "Notification settings".

**States.** Unread (drawn); all read (no "New" markers, bell count hidden); empty ("No notifications" / "Replies, mentions and shares appear here."); emails off (default) or on per user and per document. Access-removed rows are not links.

**Behaviour.** Opt-in emails per user (E18f); the bell count updates live while a shared document is open, otherwise on focus. Notifications never include document pages.

**Accessibility.** The bell’s label includes the unread count; unread is shown by the word "New" as well as the dot. Rows are real links. Focus ring drawn on the first row (inset). Escape closes the popover and returns focus to the bell.

**Platform differences.** Web and desktop identical; desktop may also use system notifications if the user allows them. Phone: the bell lives on the Files screen; the list is a full-height sheet.

**Artboards.** `H07-notifications.dc.html`, `H07-email.dc.html`.

**Todos.** [E18f](../../todos/ready/E18f-sharing-and-notifications.md).

## H08 Pricing and promises page

**Purpose.** The docs-site page that publishes the plans and prices the owner sets, the fair-pricing promises, and what needs an account.

**Entry points.** Site navigation "Pricing"; links from H02 ("Pricing and promises"), H11 ("Compare plans") and the app’s About dialog.

**Layout.** Docs-site page (not the app shell): site header with wordmark, nav (Download, Docs, Pricing, Performance, Privacy) and "Open the web app"; content max 1120 px. Sections: heading and lead; two plan cards; a terms line; "Our promises" band on `surface`; "What needs an account" table; footer.

**Content and controls.**

- Plan cards: `[PLAN NAME]`, `[PRICE]`, `[BILLING TERMS]`, one sentence on who it is for, an includes list (decided facts plus placeholders), "Buy [PLAN NAME]". Card one is on-device ("No account needed"; "Works offline; checked by a licence key on your device"); card two adds the hosted service (cloud storage `[QUOTA]`, sync, shared review). The number of plans and what each includes are placeholders for the owner.
- Terms line: currency, tax, trial terms, last-changed date and a link to the pricing history (supports "no changed deals").
- Promises (from U5a, worded narrowly; the owner approves the final text): the price shows before you start; no watermarks; what you bought stays bought; local work needs no account; your documents are for your work, no ads; the code stays MIT OR Apache-2.0.
- "What needs an account" table: four rows of tasks with Account yes/no and the plan that includes them.

**States.** One static page. Until U5a is merged, every bracketed value stays a placeholder; nothing on the page claims a price, quota or trial.

**Behaviour.** Plain links; buying goes to the payment flow (out of scope here).

**Accessibility.** One `h1`, sections labelled by their headings; the table has row and column headers; nav marks the current page with `aria-current`; contrast as in the design language; layout reflows to one column on phones. Focus ring drawn on the first card’s button.

**Platform differences.** A web page only; it works at phone width (grids use `auto-fit`).

**Artboards.** `H08-pricing.dc.html`.

**Todos.** [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).

## H09 Benchmark page

**Purpose.** Publish the release gates and the reproducible reader benchmark (method, machines, versions, raw data) so speed is proved, not claimed (principle 7).

**Entry points.** Site navigation "Performance"; link from H08.

**Layout.** Same docs-site frame as H08. Sections: heading and lead; "Release gates" table; "Reader benchmark" with file tabs, a machine select and a results table; "Method" and "Rerun it" side by side.

**Content and controls.**

- Release gates (from [UI/UX §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7)): web first page ≤ 700 ms (stretch ≤ 500 ms for 10 MB), desktop cold start < 300 ms, installer < 15 MB, longest main-thread task while scrolling ≤ 50 ms (stretch INP ≤ 200 ms p75, commands ≤ 100 ms); lamina’s results are `[n ms]` / `[n MB]`.
- Benchmark: run date, median of 3, versions; tabs for the five reference files (10-page text, 1,000-page textbook, architectural drawing, 500 MB scan, tagged two-column paper); machine select (Windows laptop, Apple-silicon Mac, `[SPEC]`); table columns cold start to first page, warm open, scroll 100 pages (long tasks), peak memory, app size, and a "Raw data" link per row. lamina’s row is tinted. A reader whose licence forbids publishing gets "Not published: its licence terms don’t allow it" with "Licence notes"; a reader not available on the chosen machine says so.
- Method bullets; "How to rerun the benchmark" (primary link to `bench/readers/README.md`), "Download results (JSON)".

**States.** Placeholders until U5b measures; stale run (older than the current release): a `warning` notice above the table, "These results are from lamina [VERSION]. A new run is due."

**Behaviour.** Tabs and the select swap the table; the URL keeps the selection so a result can be linked.

**Accessibility.** Tables have captions saying what is shown and "lower is better"; tabs are a `tablist`; numbers are tabular. No chart until there is data (a chart, when added, follows the dataviz rules and keeps the table).

**Platform differences.** A web page only; tables scroll horizontally in their own box on phones.

**Artboards.** `H09-benchmark.dc.html`.

**Todos.** [U5b](../../todos/ready/U5b-reader-benchmark.md).

## H10 Telemetry opt-in and what we collect

**Purpose.** Ask once whether to send content-free daily usage counts, show exactly what would be sent, and give a permanent setting with the last report visible.

**Entry points.** The one-time question appears on the Files home (never while a document is opening or being edited), after the app has been used on a few separate days and not in v0.1; Settings › Privacy (`settings.privacy`); palette "Usage counts". No shortcut.

**Layout.** Web: dialog over the Files home. Desktop: the Settings window (macOS frame drawn) with the settings nav and the Privacy pane.

**Content and controls.**

- Question: "Help decide what lamina builds next?", what is and is never sent, an open `details` "Exactly what is sent" with the JSON payload (day, app version, platform family, counts per mode and per command id), when it is sent, that the network indicator counts it, no IP retention, `[RETENTION]`, where to change it. Two equal secondary buttons, "Don’t send" and "Send usage counts" (no primary, so neither is nudged); "Privacy page" link.
- Privacy pane: "Send crash reports" switch (D7a, off) with its description; "Send usage counts" switch (`telemetry.toggle`) with description, last sent / next / kept for, "What was last sent" (the actual last payload), "Copy report", "How usage counts work", and that reports carry no identifier.

**States.** Off (default); on (drawn on desktop); question answered (never asked again; the setting stays); offline (counts wait and are sent once, never backfilled per day beyond [RETENTION]).

**Behaviour.** Opting out stops all sending immediately; nothing is sent while off (U5c acceptance criteria). Counts are aggregated per day on the device; the payload schema has no free-text fields.

**Accessibility.** Switches are `role="switch"` labelled by their headings and described by their text; the payload is real text in a `pre`, readable by screen readers; initial focus on "Don’t send" (ring drawn); desktop ring on the usage switch.

**Platform differences.** Web: question as a dialog, setting in Settings › Privacy. Desktop: sent from the main process next to the crash-report consent (U5c); the pane drawn. Windows/Linux: same pane in a native window frame. Phone: the question is a large-detent sheet.

**Artboards.** `H10-telemetry.dc.html`, `H10-telemetry-desktop.dc.html`.

**Todos.** [U5c](../../todos/ready/U5c-opt-in-usage-telemetry.md).

## H11 Upgrade prompt before work starts

**Purpose.** When a command is outside the user’s plan, say so before any input, upload or change, with the plan name and price; never after work.

**Entry points.** Choosing such a command from the palette, a menu or a toolbar. The drawn example is "Convert to Word…" (a server conversion, illustrative only: which features belong to which plan is the owner’s decision in U5a).

**Layout.** Plan label: inside the command palette row (and the same badge in menus and tool tooltips). Prompt: dialog (480 px) over the reader.

**Content and controls.**

- Palette row: command, group, and a badge "In [PLAN NAME]" in place of a shortcut chip; the palette footer explains the label. Commands in the user’s plan show no badge. No lock icons on anything the user already uses.
- Prompt: title "Convert to Word is part of [PLAN NAME]"; "Your current plan doesn’t include it. Nothing has started: your file is unchanged and nothing has been uploaded."; a plan block (`[PLAN NAME]`, `[PRICE] [BILLING TERMS]`, what it includes, and for server features "You’ll be asked before the file is uploaded"); "Everything you can do today keeps working whether you upgrade or not."; "Compare plans" link, "Not now" (secondary, initial focus), "Get [PLAN NAME]" (primary, `plan.upgrade`).

**States.** Prompt (drawn); label in the palette (drawn); already on the plan (no badge, no prompt, the command runs); offline (the prompt still shows the price from the last known price list, and "Get [PLAN NAME]" says it needs a connection); after buying, the original command starts from the beginning, with its own consent step if it uploads.

**Behaviour.** The check runs when the command is chosen, before its dialog or tool opens, so no form is filled and no file is edited first. Dismissing changes nothing. The prompt never appears on save, export or print of work already done.

**Accessibility.** The badge is text, linked to the option with `aria-describedby`; the prompt is a labelled dialog; focus returns to the palette or menu item on "Not now".

**Platform differences.** Web and desktop identical. App-store builds (if any) use the store’s purchase sheet after the same prompt (U5a risk note). Phone: the prompt is a large-detent sheet; the badge appears in the More sheet rows.

**Artboards.** `H11-upgrade-prompt.dc.html`, `H11-tier-label.dc.html`.

**Todos.** [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).

## New components

Built from the design language tokens; values are **proposed** until a reviewed board replaces them.

| Component | Spec | Used in |
| --- | --- | --- |
| Page heading | Newsreader 28/34/500 (`h1` of settings and list pages); docs-site pages use 48/1.1/400 and section headings 30–34/400 | H02, H03, H08, H09, H10 |
| Settings nav | 220 px column, `hairline` right, padding `16px 10px`, links 32 px high, radius 4; current `accent` 600 on `accent-tint`, `aria-current="page"` | H02, H10 |
| Avatar and presence stack | 24–48 px circle, initials 11–16/600; own avatar `accent-tint` + `accent`; reviewers get a presence colour with white initials (≥ 4.5:1: #2F6A45, #8A5300, then `accent`, #9B2335); stack overlaps by 6 px with a 2 px `ground` ring | H02, H05, H06, H07 |
| Remote selection tag | 2 px outline in the reviewer’s colour, offset 2, with a 20 px name tag (white 12/600 on the colour, radius 3) at the top right | H06 |
| Sync status item | Status-bar button (min-height 24, 12 px) with sync, cloud-off or warning icon and text: "Live · comments synced 10:32", "2 saves waiting to upload", "Conflict copy · Details"; opens its popover or dialog | H04, H06 |
| Status popover | Non-modal `role="dialog"` anchored to a bar item: `surface`, `chip-border` edge, radius 6, menu shadow, padding 16, gap 12, title Heading 18 | H04, H07 |
| Lease and offline notices | The §7.20 notice as a full-width strip under the toolbars (min-height 36, padding `6px 16px`, `hairline` bottom): `accent-tint` for a lease, `surface` for offline; action buttons 28 high on the right | H04, H06 |
| File table with sync badges | Real `table`; name in Newsreader 16; badges §7.23 with an icon each; inline progress (4 px bar, 160 px) for uploads | H03 |
| Consent definition list | `dl` in a 112 px / 1fr grid between `hairline` rules; terms 12/600 `text-3` | H03 |
| Licence key field | Read-only input in a monospace stack (`ui-monospace, 'SF Mono', Menlo, Consolas`), masked except the last group, with Show / Copy key | H02 |
| Payload block | `pre` on white, `control-border`, radius 4, padding `12px 14px`, 13/1.5 monospace, scrolls horizontally | H10 |
| Notification row | Link row, padding 10, radius 4: avatar, sentence with the actor in 600, excerpt 13 `text-2`, context 12; unread = 8 px `accent` dot + "New" 12/600; unread rows on `accent-tint` | H07 |
| Bell with count | 28 px icon button; count pill 14 px high, `accent` with white 10/600 tabular; the count is also in the `aria-label` | H07 |
| Plan block and card | Plan block (in dialogs): white, `hairline` edge, radius 4, padding 16: plan name Newsreader 18, price 600 + terms `muted`, check-list 13. Plan card (docs site): `surface`, padding 28, name 26, price 40 Newsreader | H02, H08, H11 |
| Plan label | Badge "In [PLAN NAME]" (§7.23 neutral; on the active palette row: white with `accent` border and text) in place of a shortcut chip | H11 |
| People row | 48 px row: avatar, name over email, role select (28 high), remove icon button | H05 |
| Docs-site header | 64 px, max 1120 px: wordmark Newsreader 24/500, nav 15 px links (current: 600 + 2 px `accent` underline), one secondary button | H08, H09 |

New icons (same grid and stroke as §5; add them to the design language table when reviewed):

| Name | Markup |
| --- | --- |
| Cloud | `<path d="M7.5 18.5h9.5a3.5 3.5 0 0 0 .4-7A5.5 5.5 0 0 0 6.8 10 4.3 4.3 0 0 0 7.5 18.5z"></path>` |
| Cloud off | Cloud + `<path d="M4 4l16 16"></path>` |
| Sync | `<path d="M5 11a7 7 0 0 1 12.5-3.5M19 13a7 7 0 0 1-12.5 3.5"></path><path d="M17.5 4v3.5H14M6.5 20v-3.5H10"></path>` |
| Bell | `<path d="M6.5 16.5V11a5.5 5.5 0 0 1 11 0v5.5l1.5 1.5H5z"></path><path d="M10 20.5h4"></path>` |
| Link | `<path d="M10 14a3.5 3.5 0 0 0 5 0l3-3a3.5 3.5 0 0 0-5-5l-1 1"></path><path d="M14 10a3.5 3.5 0 0 0-5 0l-3 3a3.5 3.5 0 0 0 5 5l1-1"></path>` |
| Download | `<path d="M12 4.5V15M8 11l4 4 4-4"></path><path d="M5 19.5h14"></path>` |
| Laptop | `<rect x="5" y="5.5" width="14" height="10" rx="1"></rect><path d="M3 18.5h18"></path>` |
| Phone | `<rect x="7.5" y="3.5" width="9" height="17" rx="1.5"></rect><path d="M11 17.5h2"></path>` |

Upload uses the existing Share glyph.
