# Paper: account, cloud, collaboration and product pages

- **Status:** Draft (Paper direction, 2026-10-29)
- **Canvas:** [lamina Paper — Account, cloud and product](https://claude.ai/artifact/Ronx2eZUMeMbgZGbx8CYhX)
- **Inventory:** [screens.md, section H](screens.md#h-account-cloud-collaboration-and-product-pages-h-e18-u5) · **Look and components:** [design-language.md](design-language.md)
- **Plans:** [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and [optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features), [roadmap: decisions made](../../plans/roadmap.md#decisions-made), [UI/UX principle 10](../../plans/ui-ux.md#4-principles) and [§5.9](../../plans/ui-ux.md#59-product-promises-r18-r19)

This area covers everything that involves our server or our prices: the optional account (H01, H02), cloud documents and sync (H03, H04), shared review (H05–H07), the product pages on the docs site (H08, H09), opt-in usage counts (H10) and the upgrade prompt (H11).

Rules every screen here follows:

- **Local use never needs an account.** Sign-in is offered, never required; nothing in the reader, editor or save path checks an account ([roadmap: hosted service](../../plans/roadmap.md#decisions-made), H2 acceptance criteria).
- **Nothing leaves the device without a visible, per-document choice.** A document goes to the cloud only after its consent step; from then on every save of that document uploads a new version, and the top bar's save state says so ("Saved · uploaded to your cloud", "Save uploads a new version"). "Stop syncing this document" in More ends it. Every request our server receives is counted by the network indicator, which names the destination.
- **Destination names** (the network indicator and the consent blocks use exactly these): "lamina’s account service" (`/api/auth/*`, H01, H02), "lamina’s storage" (host `storage.lamina.example`, H03, H04), "lamina’s review server" (live connection, H05–H07), "lamina’s job server" (host `jobs.lamina.example`, server jobs in Convert X02), "lamina’s usage-count server" (H10). Hosts are placeholders until the deployment is named.
- **Screens without a document** use the R01 home ([reader-a.md R01](reader-a.md#r01-home--open)): its top bar (wordmark, Commands, More) and its title treatment. Signing in adds the Cloud and Shared with me tabs, the bell and the account avatar to that home; there is no separate "Files" screen.
- **Priced, never hostile** ([principle 10](../../plans/ui-ux.md#4-principles)): the plan and price show before work starts; no surprise watermarks; no changed deals. Pricing is not decided, so every price is `[PLAN NAME]`, `[PRICE]`, `[BILLING TERMS]`, `[QUOTA]`; the owner fills them in [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).
- **Copy that makes a commitment is a draft.** Promise text on H02 and H08 is marked `[PROMISE TEXT — OWNER TO APPROVE]`; the owner words and approves each promise in [U5a](../../todos/ready/U5a-pricing-and-promises-page.md) (and its intro sentence). The tier mapping (which tasks are in which plan, whether there is a free tier) is also the owner’s: drawn as `[PLAN NAME or FREE]`.
- Other sentences that make a commitment and wait for U5a are marked the same way, with the sentence inside the bracket so it never reads as approved copy (`[PROMISE TEXT — OWNER TO APPROVE: <sentence>]`): H11 "Everything you can do today keeps working whether you upgrade or not.", the H03 consent row "stored for your own use and never used for anything else", every H08 promise. How a licence is checked (offline key or store purchase, UI/UX open question 1) is `[HOW THE LICENCE IS CHECKED: OFFLINE KEY OR STORE PURCHASE]` on H02 and H08, and the H02 Licence page’s "nothing is sent" line is prefixed "If keys are checked offline:".
- Command ids use the shared namespaces (`file.*`, `account.*`, `licence.*`, `settings.*`, `comment.*`, `edit.*`); the cloud, sync and sharing commands live under `file.*` because they act on the open or listed file.
- Other undecided values are bracketed too: `[REGION]` (data residency), `[N]` (days to delete an account), `[RETENTION]` (usage counts), `[LAMINA DOMAIN]`, `[VERSION]`, `[SPEC]`, and every benchmark number (`[n ms]`, `[n MB]`).

Artboards live on the canvas; web screens are 1440-wide PAGE boards (900 high; the docs-site pages H08 and H09 are 1900 and 1680 high because they are long pages), phone boards are 390 × 844. Every board has the `accent` tweak (Teal, Blue, Plum, Green, [design language §2.8](design-language.md#28-accent-colours)).

## H01 Sign in / create account

**Purpose.** Let someone who wants cloud storage, sync or shared review sign in, without suggesting that anything else needs it.

**Entry points.** More menu › "Sign in…" (`account.sign-in`); the "Sign in" text button on the R01 home top bar (signed out); palette "Sign in"; the H05 share dialog and the H03 Cloud tab when signed out ("Sign in to use the cloud"). No shortcut.

**Layout.** Web: a dialog (design language [§7.18](design-language.md#718-dialog)) over the dimmed reader; the reader keeps its top bar, page and status bar. Phone: the same content in a bottom sheet at the large detent over the R01 phone home (left-aligned wordmark and More; the root screen, so no Back button), with the grabber button ("Resize sheet") and actions full width.

**Content and controls.**

- Title "Sign in to lamina"; close icon button ("Close sign in").
- One sentence that local use needs no account and what an account adds: "You don’t need an account to open, edit or save files on this device." ("this phone" on touch), then what signing in adds.
- Email field (label "Email", `autocomplete="email"`), help text "We’ll email you a one-time sign-in code. Your open document stays on this device."
- "Not now" (secondary, closes) and "Continue" (primary, `account.request-code`).
- Footer: "Create an account" link (`account.create`, same flow with a name field), "What an account stores" (privacy page).

**States.**

- Email entry (drawn).
- Code sent: the field becomes "Sign-in code" (6 digits, `autocomplete="one-time-code"`, numeric keypad on touch), with "Send a new code" ghost and the address shown above it.
- Wrong or expired code: error text under the field in `danger` ("That code has expired. Send a new one."), `aria-invalid`.
- Offline: the dialog explains "Signing in needs a connection. Everything on this device keeps working." and "Continue" is disabled with that reason.
- Signed in: the dialog closes, a toast says "Signed in as r.osei@northwind.example"; the top bar of the home shows the bell and the account avatar.
- The sign-in method (email code, password, passkey, providers) depends on the [H1 auth spike](../../todos/ready/H1-auth-spike.md); the drawing assumes an email code, the layout takes extra provider buttons above the email field.

**Behaviour.** Sign-in never touches the open file and never reloads the page. Requests go to our origin only (`/api/auth/*`) and are counted by the network indicator. Escape or "Not now" closes and returns focus to the opener.

**Accessibility.** Focus starts in the email field (ring drawn). Dialog (and the phone sheet) `role="dialog"`, `aria-modal`, labelled by the title and described by the intro sentence (`aria-describedby` on both). Errors are linked with `aria-describedby`. Phone inputs are 16 px and 44 px high.

**Platform differences.** Web: in-app dialog, cookie session. Desktop: "Continue" opens the system browser (H2: deep-link callback, token in the OS keychain); the dialog shows "Finish signing in in your browser" with "Open the browser again" and "Cancel" until the callback arrives. Phone: sheet as drawn. Tablet: the web dialog with touch sizes.

**Artboards.** `H01-sign-in.dc.html`, `H01-sign-in-phone.dc.html`.

**Todos.** [H2](../../todos/ready/H2-accounts-and-login.md).

## H02 Account and plan

**Purpose.** One place for the account: who is signed in, the plan and its price, the licence key bought with the account, signed-in devices, data export and deletion. A second page, Settings › Licence, takes a key bought elsewhere and works signed out and offline, so the "no account needed" plan can be activated without one.

**Entry points.** Account avatar › "Account and plan" (`account.open`); Settings › Account and plan; palette "Account and plan". Licence: Settings › Licence, About › "Enter a licence key…", palette "Enter a licence key" (`licence.enter`), and the H11 prompt for someone who already bought a key. No shortcuts.

**Dependency.** Licence checks are out of U5a’s scope and wait for the entitlement todo U5a will add; UI/UX [open question 1](../../plans/ui-ux.md#8-open-questions) allows offline keys or store purchases. If the owner picks store purchases, the Licence page becomes "Restore purchases" and the key block on the account page goes away; the layout is otherwise unchanged.

**Layout.** Settings page: top bar with Back ("Back to Q3-board-report.pdf", or "Back to home" with no document) and "Settings"; a 220 px settings nav on the left (200 in the desktop Settings window) (`<nav aria-label="Settings sections">`, current item `accent-tint`, `aria-current="page"`) with the one section list shared with the desktop Settings window ([desktop-shell.md S07](desktop-shell.md#s07-settings-window)): General · Appearance · Updates · Privacy · Shortcuts · Licence · Account and plan · Notifications · Advanced · About. The web app has no Updates or Advanced section, so its nav reads General · Appearance · Privacy · Shortcuts · Licence · Account and plan · Notifications · About; content on `surface`, up to 960 px wide in a two-column grid that becomes one column below ~900 px; the status bar keeps the network indicator.

**Content and controls.**

- Header: avatar, name (page heading), email and "signed in since"; "Edit name or email" (secondary), "Sign out" (ghost, `account.sign-out`).
- **Plan:** `[PLAN NAME]` with an "Active" badge, `[PRICE] · [BILLING TERMS] · bought <date>`, what it includes, and a promise line drawn as `[PROMISE TEXT — OWNER TO APPROVE]` (what happens to this purchase if plans change). Actions: "Change plan" (`account.change-plan`), "Receipts" (`account.view-receipts`), "Pricing and promises" (H08).
- **Licence key for offline use:** mirrors only a key bought while signed in. Read-only masked key field in the `mono` token, "Show" (`aria-pressed`, label "Show licence key"), "Copy key" (`licence.copy-key`), "Download licence file" (`licence.download-file`). Copy says that features on the device never need sign-in, how the key is checked as `[HOW THE LICENCE IS CHECKED: OFFLINE KEY OR STORE PURCHASE]` (undecided, see Dependency), and that a key bought elsewhere goes in Settings › Licence.
- **Settings › Licence (signed out, `H02-licence-key`):** "No licence on this device yet" intro, ending with "You don’t need an account." and the same `[HOW THE LICENCE IS CHECKED: …]` placeholder; "Enter a licence key" block with the key field (mono), "Activate" (primary, `licence.enter`) and "Open licence file…" (`licence.open-file`); "What happens": checked on this device, stored on this device only, nothing sent (the network indicator stays at 0), `[DEVICE TERMS, IF ANY]`; links to pricing and to Account and plan.
- **Signed-in devices:** one row per device (icon, name, app, last active); the current one has a "This device" badge; others have "Sign out" (`account.sign-out-device`), each named for its device ("Sign out MacBook Air"). Note: "Signing a device out stops its sync. Files saved on it stay there."
- **Appearance:** a read-only row with the current values ("Light · Teal") and the line "Saved to your account, so every device you sign in on matches. Change it in View settings." There is no switch: Theme and Accent always sync while signed in ([§2.8](design-language.md#28-accent-colours) has the first-sign-in rule).
- **Your data:** cloud storage used ("5 documents · 18.3 MB of [QUOTA]", matching H03; the bar uses `aria-valuetext` because the quota is unknown), "Export my data" (`account.export-data`), "Delete account…" (`account.delete`, danger ghost).

**States.**

- No plan: the Plan block shows "No plan" and "See plans" instead of the badge; the licence block explains how to add a key bought elsewhere ("Enter a licence key…").
- Payment failed or plan ended (if the model has renewals): a `warning` notice in the Plan block with the date on which hosted features stop; on-device features bought outright keep working.
- Key copied: toast "Licence key copied".
- Delete account: an `alertdialog` that names what is deleted (account, cloud documents, shares) and what is not (files on devices), with a typed confirmation and the danger-filled "Delete account".
- Export: progress in place, then a download.
- Licence page: entering (drawn); valid: the block becomes "[PLAN NAME] · active on this device" with "Remove licence from this device"; invalid: field error "This key isn’t valid. Check for missing characters." (`danger`, `aria-invalid`); device limit reached (only if the terms have one, checked when next online and never blocking offline use): `warning` notice naming the devices and "Manage devices".

**Behaviour.** Nothing here changes a document. Signing out a device revokes its session; its next sync attempt asks it to sign in again. Plan changes go to the payment flow (outside lamina’s UI; to be designed with the payment todo U5a will add).

**Accessibility.** Each block is a `section` with a heading; the key field has a real label; "Show" announces its state ("Show licence key", `aria-pressed`). One focus ring drawn (Copy key).

**Platform differences.** Web: as drawn. Desktop: the same pane inside the Settings window (S07's metrics: content on `surface`, padding `28px 32px`, Title Newsreader 20); "Download licence file" becomes "Save licence file…" (system save dialog), and a desktop that is signed out can still "Enter a licence key…". Phone: the settings nav becomes a list screen; each block is its own screen.

**Artboards.** `H02-account.dc.html`, `H02-licence-key.dc.html`.

**Todos.** [H2](../../todos/ready/H2-accounts-and-login.md), [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).

## H03 Cloud documents and upload consent

**Purpose.** Show the documents a signed-in user chose to upload, with each one’s sync status, and make every upload an explicit, informed choice.

**Entry points.** Home (R01) › Cloud tab (`file.open-cloud-list`); "Upload a file…" on that tab (`file.upload-to-cloud`); More › "Save to cloud…" in the reader (`file.upload-to-cloud`); the share dialog for a local file ("Upload to share"). No shortcuts.

**Layout.** Web: the R01 home once signed in: R01's top bar (wordmark "lamina" Newsreader 18/500; right: Commands, the bell with its count pill, the account avatar, More), then tabs "On this device · Cloud · Shared with me" (On this device is R01's own content: drop zone, Recent, privacy line), a header row (page heading, one-line explanation, storage meter, primary "Upload a file…") and a table. Upload consent: a 560 px dialog over the reader. Phone: R01's phone top bar (left-aligned wordmark Newsreader 20/500) with the bell, the avatar and More (44 px each), then the same three tabs (44 px), "On this phone · Cloud · Shared with me" ("this phone" replaces "this device" on touch), storage line, a list of 72 px rows and a docked "Upload a file…" button above the home indicator.

**Content and controls.**

- Files and sizes match H02 (5 documents, 18.3 MB) and H04: the conflict copy listed is the one H04 creates.
- Table columns: Name (opens the document, `file.open`), Sync, Size, Last saved (time · device), and a More button per row (Open, Download, Rename, Share…, Remove from cloud: `file.download-cloud-copy`, `file.rename`, `file.share-for-review`, `file.remove-from-cloud`).
- Sync statuses, each a badge with an icon and a word (never colour alone): **Synced** (positive), **Uploading… 62%** (progress bar, "Cancel upload"), **Newer on MacBook Air** (neutral, sync icon: another device saved a newer revision; opening downloads it), **Conflict copy** (warning, see H04), **Waiting to upload** (neutral, offline), **Shared with 3** (accent, added beside the sync badge).
- Footnote: "The web app syncs a document when you open or save it. The desktop app also syncs in the background."
- Upload consent dialog: the design language's server consent block ([§7.18](design-language.md#718-dialog)), the same pattern as Convert X02. Title "Upload Q3-board-report.pdf to your cloud?"; inside the white block: Share icon, "Cloud storage is on lamina’s server", "Nothing has been uploaded yet. Nothing is sent until you agree below."; the four fixed rows **What is sent** (the file as last saved, with its comments and form values · 2.4 MB · 24 pages), **Where to** (lamina’s storage (`storage.lamina.example`), in `[REGION]`, over HTTPS; only you until you share), **How long it stays** (until you remove it from the cloud; each save of this document uploads a new version; the local copy stays), **What it is used for** (`[PROMISE TEXT — OWNER TO APPROVE: stored for your own use and never used for anything else.]`); then the checkbox "Upload Q3-board-report.pdf to my cloud, and upload each new version when I save it" (unchecked, never remembered). Footer: "Privacy page" link, the hint "Tick the box to upload", "Keep on this device" (secondary), "Upload" (primary, `aria-disabled` until the box is ticked).
- Per-document consent, unlike X02's per-job consent: the box names the later uploads too, because storage keeps the document in step with its saves; a server job uploads once.

**States.**

- Signed out: the Cloud tab shows an empty state, "Your cloud documents appear here" / "Sign in to upload documents you choose. Nothing is uploaded otherwise." with "Sign in".
- Empty (signed in): "No cloud documents yet" / "Upload a file, or choose Save to cloud from a document’s More menu."
- Unsaved changes when uploading: the consent dialog says "Your 1 unsaved change isn’t included. Save first to include it." and offers "Save and upload".
- Quota nearly full (≥ 90%): the meter turns `warning` and a notice above the table names what is using the space; an upload that doesn’t fit is refused before it starts, with the size and the space left.
- Upload failed: row badge "Upload failed" (danger) with "Retry".
- Stop syncing: More › "Stop syncing this document" (`file.stop-syncing`, in the reader and on the row) asks whether to keep the cloud copy; afterwards saves stay on the device and the save state drops "uploaded to your cloud".

**Behaviour.** Nothing uploads without the consent dialog the first time a document goes to the cloud; afterwards each save of that document uploads a new revision (H3: incremental saves dedupe by hash). "Remove from cloud" deletes the cloud copy and its revisions after an `alertdialog`; the local file is not touched. The network indicator counts uploads and names our storage as the only destination.

**Accessibility.** A real `<table>` with column headers; per-row More buttons are labelled with the file name; progress bars have `role="progressbar"` and values and sit outside the row's link (phone); tabs use `role="tablist"`. The consent dialog is described by "Nothing has been uploaded yet…" (`aria-describedby`); the checkbox label repeats the file name; its initial focus is on "Keep on this device" (ring drawn there), and Upload stays disabled until the box is ticked, so Enter never uploads by accident.

**Platform differences.** Web: syncs on open and save (OPFS cache). Desktop: the Cloud tab adds "Sync now" and shows background progress; the consent dialog adds "Keep the file on this computer as the original" (on by default; H3: local files stay the source of truth unless the user opens the cloud copy). Phone: drawn; row actions open a bottom sheet. Tablet: the web table with 44 px rows.

**Artboards.** `H03-cloud-documents.dc.html`, `H03-upload-consent.dc.html`, `H03-cloud-documents-phone.dc.html`.

**Todos.** [H3](../../todos/ready/H3-cloud-document-storage.md), [H4](../../todos/ready/H4-document-sync.md).

## H04 Sync conflict and offline edits

**Purpose.** When two devices save from the same revision, keep both and say so plainly; when offline, keep saving locally and show what will upload later. Whole-file revisions only; live co-review is H06 (H4 scope).

**Entry points.** The conflict dialog appears after a save is rejected because its base revision is stale. The status-bar item "Conflict copy · Details" and the H03 "Conflict copy" badge reopen it (`file.show-sync-conflict`). The offline item "2 saves waiting to upload" opens the queue popover (`file.show-upload-queue`).

**Layout.** Both boards are in Comment mode with the Comment toolbar of design language §8.1 (Text box ▾ with its menu chevron) and the editor top bar (Undo/Redo after the save state, View after Find). The save state names the destination: "Saved · uploaded to your cloud" (conflict board) and "Saved on this device · uploads when you’re online" (offline board); with changes it reads "1 unsaved change · Save uploads a new version". The sync item sits in the status bar’s left group next to the network indicator, never in the middle slot. Conflict: dialog (560 px) over the reader, which now shows the conflict copy as the open file. Offline: a `surface` notice strip under the secondary toolbar ("You’re offline. …") and a popover anchored to the status-bar item.

**Content and controls.**

- Conflict dialog: title "Q3-board-report.pdf changed on another device"; one sentence that both versions were kept; two side-by-side blocks, "Your version · this browser" (new name `Q3-board-report (conflict, this browser, 29 Oct 2026).pdf`, saved time, a summary of changes, "Open now" badge) and "MacBook Air’s version" (keeps the original name, saved time, changes); a hint on how to end up with one file. Actions: "Compare the two" (`compare.open` with both files preselected, from the compare area), "Open MacBook Air’s version" (`file.open-other-version`), "Keep working on my copy" (primary, closes).
- Offline notice: cloud-off icon, "**You’re offline.** Saves stay on this device and upload when you’re back online. Reading, commenting and saving all keep working."
- Status bar: network indicator "0 network requests for this document · offline"; sync item "2 saves waiting to upload".
- Queue popover: "Waiting to upload", one row per queued save (time, change summary), the rule that a later conflict keeps both, "Download a copy" (secondary) and "Upload now" (`aria-disabled` while offline, reason under it).

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

**Entry points.** "Share" button in the top bar (shown on every board of a signed-in user’s cloud document: H04, H05, H06, H07), More › "Share for review…" (`file.share-for-review`), H03 row More › "Share…". No shortcut.

**Layout.** Dialog (560 px) over the reader in Comment mode (§8.1 Comment toolbar, Undo/Redo, View, full top bar; save state "Saved · uploaded to your cloud"). People’s avatars use the presence colours, so a reviewer has the same colour here and in H06.

**Content and controls.**

- Title "Share for review"; intro: who can open what, and that comments stay beside the PDF until saved into it (base PDFs are immutable; ADR 0012 via [E18f](../../todos/ready/E18f-sharing-and-notifications.md)).
- "Add people by email" field, a role select (Can comment by default; Can view; Can edit), "Invite" (primary, `file.invite-reviewer`).
- People list: owner (no controls), each member with a role select (`file.change-reviewer-role`) and a remove button ("Remove A. Patel", `file.remove-reviewer`); pending invites say "Invited 28 Oct · not opened yet".
- Role help: "Can view: read and see comments. Can comment: also add comments and replies. Can edit: also change form values, page order and text, one person at a time for text."
- Review link: switch (`file.toggle-review-link`), read-only link field, link role select, "Copy link" (`file.copy-review-link`), and "Turning the link off, or removing a person, ends their access at once, even if they have the document open."
- "Email me about new comments and replies on this document" checkbox (`settings.comment-emails`, per document, off by default; drawn on for Q3-board-report.pdf, matching H07), "Done".

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

**Entry points.** Opening a shared cloud document (from H03, H07 or a review link). The presence button lists who is here (`comment.show-reviewers`).

**Layout.** The normal shell in Edit mode, with the canonical Edit toolbar of design language §8.1 (Select · Edit text · Add text · Image · Shape ▾ · Link │ Page marks ▾ · Replace… │ Redact · Protect · Tools ▾). Top bar: save-state text "Shared · comments sync live · Save uploads a new version", View after Find, Undo/Redo, a presence stack (other people’s initials in their presence colour, "+ you") and "Share". Secondary toolbar for the current mode. Lease notice strip under it when needed. Inspector with the selected thread. Status bar: network indicator naming the review server and, beside it in the left group, the sync item ("Live · comments synced 10:32"); page label on the right.

**Content and controls.**

- **Presence:** each reviewer gets a colour from the presence palette (never a status colour; design language [§2.7](design-language.md#27-fixed-content-palettes)): A. Patel violet `#5B4B8A`, J. Mensah blue `#1F5C8A`. Their current selection is a 2 px dashed outline on the page in that colour with a name tag ("J. Mensah"). Your own selection keeps the solid `accent` focus outline, so the two differ whatever the accent ([§2.7](design-language.md#27-fixed-content-palettes) Presence).
- **Live thread:** the inspector shows the quoted text, messages with author and time, and a live "J. Mensah is replying…" row (`aria-live="polite"`); reply field, "Reply" (primary, `comment.reply`) and "Resolve" (`comment.resolve`).
- **Lease** ([E18g](../../todos/ready/E18g-leases-and-base-revisions.md)): drawn in Edit mode while A. Patel holds the text lease. Edit text, Add text, Image, Shape, Page marks, Replace, Redact and Protect are `aria-disabled` and point with `aria-describedby` to the context note "Text, image, page-mark, redaction and signing tools wait for A. Patel" (Select, Link and Tools ▾ stay on; signing is disabled in Fill & Sign); the notice reads "**A. Patel is editing text on page 7** (since 10:20). Text, page-mark, redaction and signing changes take turns so nobody’s work is overwritten. Comments, links and form values still sync live." with "Tell me when it’s free" (`edit.notify-when-free`).

**States.**

- Live (drawn); reconnecting: the sync item reads "Reconnecting…" and new comments queue locally; offline (desktop): "Offline · 3 comments waiting", sent on reconnect (E18e).
- You hold the lease: notice "You’re editing text. Others can comment meanwhile." with "Finish editing" (`edit.finish-editing`); saving mints a new base revision and other clients rebase.
- Lease freed: toast "A. Patel finished editing. Text tools are available."
- Redaction applied in a shared document: notice that earlier versions were purged from lamina’s storage and caches, and that copies saved outside lamina are out of reach (E18g).
- Access removed: a dialog "Your access was removed" with "Download your comments" (if allowed) and "Close"; unsent changes are discarded and the dialog says how many.

**Behaviour.** Comments, highlights, form values and page order sync as records (LWW, deletes win); undo of your own change works after remote changes (ADR 0006 §4). PDF bytes never cross the live connection. Content edits, redaction and signing need the lease or run as a server job.

**Accessibility.** The presence button’s label lists everyone ("3 people here: you, A. Patel, J. Mensah"). Presence colours always come with a name tag. New remote comments are announced politely, at most one announcement per thread per few seconds. Disabled tools explain why via the context note. Focus ring drawn on the reply field.

**Platform differences.** Web and desktop identical; desktop also works offline and syncs on reconnect. Phone: presence collapses to a count in the top bar subtitle; the thread is a medium-detent sheet; the lease notice sits under the top bar.

**Artboards.** `H06-shared-review.dc.html`.

**Todos.** [E18e](../../todos/ready/E18e-client-sync-and-presence.md), [E18g](../../todos/ready/E18g-leases-and-base-revisions.md).

## H07 Notifications

**Purpose.** Tell people about replies, shares and removed access, in the app and, if they opt in, by email.

**Entry points.** The bell icon button in the top bar, shown on every signed-in board (the home with its Cloud tab on web and phone, and any open document while signed in: H03, H04, H05, H06, H07; "Notifications, 2 unread"; `account.open-notifications`); Settings › Notifications. No shortcut.

**Layout.** In-app: a popover (menu shadow, `surface`, 400 px) under the bell. Email: a plain one-column message, 560 px content width; its artboard is a fixed 720 × 900 preview on purpose (an email is not an app page).

**Content and controls.**

- Popover: title "Notifications", "Mark all as read" (`account.mark-notifications-read`); "Today" / "Earlier" overlines; rows for replies, shares and removed access (E18f; @mentions are not in its scope and are a proposed follow-up), as links (avatar in the reviewer’s presence colour, "**J. Mensah** replied on Q3-board-report.pdf", the comment text, "Page 3 · 10:34", and a "New" marker with a dot for unread). Selecting a row opens the document at the thread and marks it read. Footer: "Email notifications are off" + "Notification settings".
- Email: subject "J. Mensah replied on Q3-board-report.pdf"; heading, context sentence, the reply text, "Open the comment" button-link, the reason it was sent ("because you turned on comment emails for this document"), a line that the email has the reply text and the document’s name but never the document or its pages, and "Turn off emails for this document" · "Notification settings".

**States.** The two boards agree: comment emails are set per document, so the popover footer reads "Comment emails: on for 1 document" and the email is for that document. Unread (drawn: both unread rows on `accent-tint` with "New"; the bell count pill is 16 px high with 12/600 text, the badge minimum of §7.23); all read (no "New" markers, bell count hidden); empty ("No notifications" / "Replies and shares appear here."); emails off (default) or on per user and per document. Access-removed rows are not links.

**Behaviour.** Opt-in emails per user (E18f); the bell count updates live while a shared document is open, otherwise on focus. Notifications never include document pages.

**Accessibility.** The bell’s label includes the unread count; unread is shown by the word "New" as well as the dot. Rows are real links. Focus ring drawn on the first row (inset). Escape closes the popover and returns focus to the bell.

**Platform differences.** Web and desktop identical; desktop may also use system notifications if the user allows them. Phone: the bell lives in the home's top bar (drawn on H03 phone); the list is a full-height sheet.

**Artboards.** `H07-notifications.dc.html`, `H07-email.dc.html`.

**Todos.** [E18f](../../todos/ready/E18f-sharing-and-notifications.md).

## H08 Pricing and promises page

**Purpose.** The docs-site page that publishes the plans and prices the owner sets, the fair-pricing promises, and what needs an account.

**Entry points.** Site navigation "Pricing"; links from H02 ("Pricing and promises"), H11 ("Compare plans") and the app’s About dialog.

**Layout.** Docs-site page (not the app shell): the docs-site header of design language [§7.39](design-language.md#739-site-and-product-pages-proposed) (64 px, wordmark Source Sans 3 24/700, 15 px nav, one secondary button) with the one site nav list for every docs-site page: Download · Docs · Pricing · Performance · Privacy, plus "Open the web app"; the download page (S09) uses §7.39's 56 px variant of the same header and should carry the same list. Footer 12 `muted` with the licence line. Content max 1120 px. Sections: heading and lead; two plan cards; a terms line; "Our promises" band on `surface`; "What needs an account" table; footer. The H08/H09 boards predate the sans-only docs site and still show Newsreader; this text wins.

**Content and controls.**

- Plan cards: `[PLAN NAME]`, `[PRICE]`, `[BILLING TERMS]`, one sentence on who it is for, an includes list (decided facts plus placeholders), "Buy [PLAN NAME]" (both cards' buttons are equal-weight secondaries, so neither plan is nudged; the owner may change this in U5a). Card one is on-device ("No account needed to buy or use it: you get [A LICENCE KEY OR A STORE PURCHASE]."; "Works offline; [HOW THE LICENCE IS CHECKED: KEY OR STORE PURCHASE]"); card two is "[WHAT THIS PLAN ADDS]" plus the hosted features (cloud storage `[QUOTA]`, sync, shared review). The number of plans, what each includes and whether they nest are the owner’s decisions.
- Terms line: currency, tax, trial terms, last-changed date and a link to the pricing history (supports "no changed deals").
- Promises (from U5a and principle 10, each drawn as `[PROMISE TEXT — OWNER TO APPROVE: <sentence>]` so no sentence reads as approved; the intro is `[PROMISES INTRO — OWNER TO APPROVE]`): the price shows before you start; no surprise watermarks (nothing added to output without telling you first: no plan drawn here adds a watermark or any mark to output; if the owner decides in U5a that a plan or trial does, the H11 prompt says so before the command or mode opens, and the export or save dialog repeats it before writing, never after); what you bought stays bought; local work needs no account; your documents are for your work, no ads; the code stays MIT OR Apache-2.0.
- "What needs an account" table: four rows of tasks with Account yes/no and the plan that includes them; the reading row is `[PLAN NAME or FREE]` because a free tier is not decided.

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
- Method bullets; "How to rerun the benchmark" (primary link to `bench/readers/README.md`), "Download results (JSON)". Rerun policy beyond "each run is dated" is `[RERUN POLICY]` (U5b only says results age quickly).

**States.** Placeholders until U5b measures; stale run (older than the current release): a `warning` notice above the table, "These results are from lamina [VERSION]. A new run is due."

**Behaviour.** Tabs and the select swap the table; the URL keeps the selection so a result can be linked.

**Accessibility.** Tables have captions saying what is shown and "lower is better"; tabs are a `tablist`; numbers are tabular. No chart until there is data (a chart, when added, follows the dataviz rules and keeps the table).

**Platform differences.** A web page only; tables scroll horizontally in their own box on phones.

**Artboards.** `H09-benchmark.dc.html`.

**Todos.** [U5b](../../todos/ready/U5b-reader-benchmark.md).

## H10 Telemetry opt-in and what we collect

**Purpose.** Ask once whether to send content-free daily usage counts, show exactly what would be sent, and give a permanent setting with the last report visible.

**Entry points.** The one-time question appears on the home (R01; never while a document is opening or being edited), after the app has been used on a few separate days and not in v0.1; Settings › Privacy (`settings.privacy`); palette "Usage counts". No shortcut.

**Layout.** Web: dialog over the R01 home. Desktop: the Settings window (macOS frame drawn) built to [desktop-shell.md S07](desktop-shell.md#s07-settings-window)'s metrics (200 px `Settings sections` nav, content on `surface` with padding `28px 32px`, Title Newsreader 20, group headings Newsreader 18) and the shared section list General · Appearance · Updates · Privacy · Shortcuts · Licence · Account and plan · Notifications · Advanced · About, Privacy open.

**Content and controls.**

- Question: "Help decide what lamina builds next?", what is and is never sent, an open `details` "Exactly what is sent" with the JSON payload (day, app version, platform family, counts per mode and per command id), when it is sent, that the network indicator counts it, no IP retention, `[RETENTION]`, where to change it. Two equal secondary buttons, "Don’t send" and "Send usage counts" (the dialog's answer to the question; the setting itself is labelled as below) (no primary, so neither is nudged); "Privacy page" link.
- Privacy pane: "Send crash reports" switch (D7a, off) with "You can view each report; nothing is sent unless you turn this on."; "Share how often each command is used" switch (`settings.usage-counts`; the same label and key as S07's Privacy section) with description, last sent / next / kept for, "What was last sent" (the actual last payload), "Copy report", "How usage counts work", and that reports carry no identifier. The payload is the same everywhere: day, app and version, platform family, counts per mode and per command id (ids from the design language's appendix, e.g. `pages.rotate-right`). S07 currently also shows "Reset anonymous ID"; that ID belongs to update checks (D5b), not to usage counts, so it moves to Updates or goes (reported to the desktop-shell owner).

**States.** Off (default); on (drawn on desktop); question answered (never asked again; the setting stays); offline (counts wait and are sent once, never backfilled per day beyond [RETENTION]).

**Behaviour.** Opting out stops all sending immediately; nothing is sent while off (U5c acceptance criteria). Counts are aggregated per day on the device; the payload schema has no free-text fields.

**Accessibility.** Switches are `role="switch"` labelled by their headings and described by their text; the payload is real text in a `pre` with `tabindex="0"` and an `aria-label` ("Exactly what is sent, scrollable"), so keyboard users can scroll it; initial focus on "Don’t send" (ring drawn); desktop ring on the usage switch. Escape on the question equals "Don’t send": nothing is sent and the question is not asked again.

**Platform differences.** Web: question as a dialog, setting in Settings › Privacy. Desktop: sent from the main process next to the crash-report consent (U5c); the pane drawn. Windows/Linux: same pane in a native window frame. Phone: the question is a large-detent sheet.

**Artboards.** `H10-telemetry.dc.html`, `H10-telemetry-desktop.dc.html`.

**Todos.** [U5c](../../todos/ready/U5c-opt-in-usage-telemetry.md).

## H11 Upgrade prompt before work starts

**Purpose.** When a command or a mode is outside the user’s plan, say so before any input, upload or change, with the plan name and price; never after work.

**Entry points.** Choosing such a command from the palette, a menu or a toolbar; choosing a mode outside the plan from the mode switcher (Comment, Fill & Sign, Organise, Edit), from the phone's Annotate and More entries, or from the status-bar item "Interactive form · 27 fields" (which opens Fill & Sign). Modes are gated like commands because the mode button is where work starts: Fill & Sign is the only way to type in a field (forms.md P01). The drawn example is "Convert to Word…" (a server conversion, illustrative only: which features belong to which plan is the owner’s decision in U5a).

**Layout.** Plan label: inside the command palette row (and the same badge in menus and tool tooltips). Mode buttons outside the plan carry "In [PLAN NAME]" in their tooltip and `aria-describedby` text (in the markup of H11-tier-label's Edit button, illustrative; the open palette hides the tooltip there), with no lock icon. Prompt: dialog (560 px) over the reader.

**Content and controls.**

- Palette row: command, group, and a badge "In [PLAN NAME]" in place of a shortcut chip (drawn for "conv": Convert to Word…, Convert to Excel… with the badge, Convert to PDF… without, since which plan holds what is illustrative); the palette footer explains the label. Commands in the user’s plan show no badge. No lock icons on anything the user already uses.
- Prompt: title "Convert to Word is part of [PLAN NAME]"; "Your current plan doesn’t include it. Nothing has started: your file is unchanged and nothing has been uploaded."; a plan block (`[PLAN NAME]`, `[PRICE] [BILLING TERMS]`, what it includes, a line on how it is bought ("No account needed to buy it: you get [A LICENCE KEY OR A STORE PURCHASE]." for an on-device plan; "Hosted features need an account. You’ll sign in after buying." for a hosted plan), and for server features "You’ll be asked before the file is uploaded"); "Everything you can do today keeps working whether you upgrade or not."; "Compare plans" link, "Not now" (secondary, initial focus), "Get [PLAN NAME]" (primary, `account.upgrade-plan`).

**States.** Prompt (drawn); label in the palette (drawn); mode outside the plan (the same prompt titled "Edit mode is part of [PLAN NAME]", shown before the mode's toolbar appears; "Not now" leaves you in the current mode); signed out (the prompt is unchanged for an on-device plan, and "Get [PLAN NAME]" goes to payment without sign-in; for a hosted plan the button reads "Sign in and get [PLAN NAME]" and runs the H01 flow first); already on the plan (no badge, no prompt, the command runs); offline (the prompt still shows the price from the last known price list, and "Get [PLAN NAME]" says it needs a connection); after buying, the original command starts from the beginning, with its own consent step if it uploads.

**Behaviour.** The check runs when the command or mode is chosen, before its dialog, tool or toolbar opens, so no form is filled and no file is edited first. Dismissing changes nothing. The prompt never appears on save, export or print of work already done.

**Accessibility.** The badge is text, linked to the option with `aria-describedby`; the prompt is a labelled dialog; focus returns to the palette or menu item on "Not now".

**Platform differences.** Web and desktop identical. App-store builds (if any) use the store’s purchase sheet after the same prompt (U5a risk note). Phone: the prompt is a large-detent sheet; the badge appears in the More sheet rows.

**Artboards.** `H11-upgrade-prompt.dc.html`, `H11-tier-label.dc.html`.

**Todos.** [U5a](../../todos/ready/U5a-pricing-and-promises-page.md).

## New components

None beyond the proposed additions below; the presence palette, `mono`, the sync status item, the Undo/Redo top bar, the bell, the licence key field, the plan block and card, the docs-site header and the code block were merged into the design language (§2.7, §3, §7.1, §7.19, §7.37–§7.39).

## Proposed design-language additions

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.
