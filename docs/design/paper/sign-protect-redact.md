# Paper: sign, protect and redact

Screens G01–G11 of the [screen inventory](screens.md#g-sign-protect-and-redact-e10e12), drawn in the [Paper design language](design-language.md): adding a signature image, signing and certifying with a certificate, validating signatures and showing changes made after signing, protecting with passwords or certificates, sanitising, and redaction from marking to the verification report. They implement the trust surfaces of the [UI/UX plan §5.7](../../plans/ui-ux.md#57-trust-surfaces-r9r13) and the editor rounds [E10](../../plans/full-editor.md#e10-signatures), [E11](../../plans/full-editor.md#e11-security-and-sanitisation) and [E12](../../plans/full-editor.md#e12-redaction).

- **Canvas:** [lamina Paper — Sign, protect and redact](https://claude.ai/artifact/QbwnMXXDikC4Fe8gwede5P) (one row per screen, a sticky note per row).
- **Sample content:** the Northwind Housing Trust Q3 board report. Two pages are invented for this area: page 24 "Board approval" (Chair Amara Okafor and Company secretary Daniel Reyes, each with a signature field and a date) and page 23 "Appendix B. Damp and mould cases over 90 days" (a table of four fictional tenants with emails on `example.org`/`example.net`, UK drama-range phone numbers `07700 900xxx`, the test card number `4111 1111 1111 1111` and the specimen NI number `QQ 12 34 56 C`). Certificates are issued by the fictional "Example Trust CA"; the timestamp server is `timestamp.example.org`.

Four rules hold across the area:

1. **Two separately named actions** (§5.7). "Add a signature image" and "Sign with a certificate" are never merged under one "Sign" button. Wherever they appear (Fill & Sign toolbar, the signature-field menu, the dialogs, the phone sign sheet) each carries the same one sentence on what it proves:
   - Add a signature image: "Puts a picture of your signature on the page; it doesn’t prove who signed."
   - Sign with a certificate: "Proves who signed and shows any change made after signing."
   - Certify document (a kind of certificate signature): "Proves you issued this document and sets which later changes keep it valid."
   The UI never calls a signature image a "digital signature" (E10c copy test).
2. **Say what saving does to signatures before it happens.** Any operation that rewrites the file (protect, remove protection, sanitise, apply redactions) names the signatures it will remove, in the dialog that starts it (ADR 0007 §3 override); signing saves the file, and says so.
3. **Prove, don't claim.** Validation results, the sanitise report and the redaction report list what was checked and how, per item, with positive/warning/danger badges that always carry a word.
4. **Marked is not removed.** Redaction marks are labelled "Marked for redaction"; nothing says "redacted" until Apply has run and the verification report has passed.

Every desktop and web board uses the editor top bar of [design language §7.1](design-language.md#71-top-bar) (Undo · Redo, page input, zoom, Find, View, Commands, Save, More; Reader joins View in Read mode) and the secondary toolbars of [§8.1](design-language.md#81-modes-and-their-secondary-toolbars). Dialog boards draw the same shell dimmed and inert (`tabindex="-1"`). Rails follow [§8.10](design-language.md#810-rail-tab-set-proposed): 220 px with up to three tabs, 280 px with four or more (three visible plus a 28 px More tabs button). Anything that changes the file runs outside Read mode ([§8.11](design-language.md#811-read-mode-is-read-only-proposed)): sanitise and remove protection open in Edit. Check boxes and radios are native inputs visually hidden inside their label with a drawn box ([§7.19](design-language.md#719-form-controls-proposed-consistent-with-the-page-input-and-note-field)); a checked or disabled state sits inside `sc-if`.

## G01 Add a signature image

**Purpose.** Put a picture of a handwritten signature (drawn, typed or uploaded) on the page, kept in a signature library on the device, and labelled as not a digital signature (E10c).

**Entry points.**
- Fill & Sign mode, secondary toolbar: **Add a signature image** (`sign.add-signature-image`).
- Clicking or pressing Enter on an empty signature field opens the **signature-field menu** with the two named actions (G01-sign-choice).
- Command palette: "Add a signature image". No shortcut.

**Layout.** Web app shell (drawn first; inventory W, D, M). Top bar with Fill & Sign active and Save; Fill & Sign secondary toolbar (Previous field · Next field · Highlight fields │ Text · Check · Date │ Add a signature image · Sign with a certificate · Form data ▾ · … · "2 of 4 required fields filled"; every button a 16 px icon plus label, the signing actions with the Signature image and Certificate icons); rail (220 px) on Pages, with Outline and Fields; canvas on page 24; status bar with the network indicator, "Not signed yet · 2 signature fields" and the page label. The capture step is a dialog (max 560 px) over the dimmed shell; after placing, the inspector opens for the placed image.

**Content and controls.**
- *Signature-field menu* (`role="menu"`, anchored under the field; G01-sign-choice): the menu with descriptions of [§7.26](design-language.md#726-menu-proposed): two items, each an 18 px icon + a 14/600 title over the 12 px one-sentence proof; a divider; "What’s the difference?" (opens help on visual vs digital signatures). The field being signed is drawn as the *target* (1 px `accent` outline on `accent-tint`), so the only 2 px ring on the board is the focused menu item.
- *Capture dialog* "Add a signature image" (G01-signature-image):
  - Tabs **Draw · Type · Upload · Saved (2)**.
  - Draw: a 180 px white pad with a baseline and the hint "Sign above the line with a mouse, trackpad, pen or finger"; **Ink** colour (Black, Dark blue); **Undo stroke**, **Clear**.
  - Type: name input and a style radiogroup (three previews; see G01 phone). Upload (Upload icon = Share): drop zone + "Choose an image…" (PNG, JPEG; white background removed, preview shown). Saved: the library as tiles.
  - **Place it as**: "A movable signature image" (Stamp or Ink annotation, default) or "Part of the page" (flattened content; shown only when the file has no certification signature, and disabled with that reason otherwise, per E10c).
  - **Keep in my signatures on this device** (on by default): "Stored in this browser only; never uploaded" (desktop: app data).
  - A neutral note: "This is not a digital signature. Anyone could add or copy this picture. To prove who signed, use Sign with a certificate" (link opens G02).
  - **Cancel**, **Place signature** (primary).
- *Placed* (G01-signature-image-placed): the image fills the field, selected with the 2 px focus outline and four 8 px square handles (§7.32); it is a button whose name says it is selected (no `aria-pressed`); a toast "Signature image placed · Undo". Inspector "Signature image": neutral badge "Image only · not a digital signature"; **What it proves**: "Nothing about who added it…"; **Saved as**; **Size**; "Need proof of who signed?" + **Sign with a certificate…**; actions **Replace…** and **Delete signature image** (danger ghost).

**States.**
- Empty pad: Place signature disabled with the reason "Draw your signature first" as help text.
- Upload error: "This image can’t be used. Choose a PNG or JPEG under 10 MB." under the drop zone (`danger`).
- Library empty: the Saved tab shows the empty state "No saved signatures yet".
- Certified file at P1/P2: placing an annotation would break the certification, so the constraint banner (design language §7.13) appears with the first placement: "Saving will invalidate the signature…", with **Save a copy**.

**Behaviour.** Placing adds one `AddAnnot` op (Stamp for image/typed, Ink for drawn strokes) or, for "Part of the page", `AddContentObject`; one undo step each. Saving is incremental (approval-signed files stay `AppendOnly`). Clicking a field fits the image to it; placing from the toolbar without a field puts it at the page centre, selected. Escape deselects.

**Accessibility.** Focus order in the dialog: close → tabs → pad → ink swatches → Undo stroke/Clear → placement radios → library checkbox → link → Cancel → Place. The pad is a `role="img"` with a live label ("Drawing area, 3 strokes"); keyboard users use Type or Upload (the non-drag alternative to drawing, WCAG 2.5.7). The placed image is a focusable button: arrow keys move it 1 pt, Shift with arrow keys 10 pt, Alt and Shift with arrow keys resize it; its name says "Signature image, Chair field. Selected" and repeats those keys. The menu is an APG menu (arrows, Enter, Escape returns focus to the field).

**Platform differences.** Desktop: same dialog; library in app data; "Choose an image…" opens the native file dialog. Phone: the capture dialog becomes a bottom sheet at the large detent with tabs Draw · Type · Photo · Saved, 16 px inputs, 44 px targets and a full-width **Use this signature** button (G01-signature-image-phone, Type tab shown: style radiogroup with Caveat, Newsreader italic and Newsreader small-caps-style previews); placing continues in G11. Tablet: desktop layout with the action bar instead of handles-only resizing; pen drawing uses pressure but stores strokes as Ink.

**Artboards.** `G01-sign-choice.dc.html`, `G01-signature-image.dc.html`, `G01-signature-image-placed.dc.html`, `G01-signature-image-phone.dc.html`.

**Todos.** [E10c](../../todos/ready/E10c-visual-signatures.md), [E10j](../../todos/ready/E10j-signature-ui.md).

## G02 Sign with a certificate

**Purpose.** Create a PAdES digital signature with an identity from the OS keychain, a smart card or token, or a PKCS#12 file, with reason, location, appearance and an optional trusted timestamp (E10j, E10i, E10e).

**Entry points.** Fill & Sign toolbar **Sign with a certificate** (`sign.with-certificate`); the signature-field menu; the link in G01's note and inspector; palette "Sign with a certificate". No shortcut.

**Layout.** Desktop window (drawn first; inventory D). Dialog (max 560) over the dimmed Fill & Sign shell (the full Fill & Sign toolbar with Sign with a certificate pressed); the target field is drawn on the page behind the scrim in the target style (1 px `accent`, `accent-tint`), so the only focus ring is on the timestamp checkbox.

**Content and controls.**
- Title and the one-sentence proof.
- **Signature field** select: existing empty fields ("Chair · page 24") plus "New field: draw one on the page…" (closes the dialog and enters a draw-a-rectangle step, then returns).
- **Sign as** radiogroup of identities, each a card: name, organisation, issuer, expiry, and the source (Keychain / Card). **Add a certificate from a file (.p12, .pfx)…** imports a PKCS#12 after asking its password.
- **Reason** select ("I approve this document", "I am the author", "I have reviewed this document", None) and **Location** (optional).
- **How it looks on the page**: a live preview of the appearance (signature image + "Digitally signed by …", time, reason) and **Show my signature image next to the details**.
- **Add a trusted timestamp** (on by default on the desktop when a server is set; on the web off until the proxy is allowed, see Platform differences): "Asks timestamp.example.org to confirm the signing time. Only a hash of the file is sent." **Change server** (opens Settings) is a link after the help text, outside the checkbox's label.
- Note "Signing saves the file." with what happens to unsaved changes.
- **Certify instead…** (left, opens G03), **Cancel**, **Sign and save** (primary).

**States.**
- No identity found: the Sign as group is replaced by an empty state "No signing certificates on this device" with **Add a certificate from a file…** and help on getting one; Sign and save disabled.
- Expired or wrong-usage certificate: its card is disabled with the reason ("Expired 3 Mar 2026", "Not allowed for signing").
- Smart card: on Sign and save, a PIN dialog (`role="dialog"`, title "Enter the card’s PIN", attempts remaining); wrong PIN shows the error under the field.
- Signing: the dialog body is replaced by a progress label ("Saving… · Signing… · Getting timestamp…"), no cancel after the hash is sent.
- Timestamp server unreachable: "The timestamp server didn’t answer. Sign without a timestamp, or try again." with both buttons; the signature is not written until the user chooses.
- Done: toast "Signed and saved", the status bar shows the G04 badge ("Signature valid"), the signature field shows the appearance.
- Certified at P1: signing is a violation; the dialog opens with a danger notice naming the certification and only **Save a signed copy…** available.

**Behaviour.** Sign and save runs: pending ops → save (incremental, or full rewrite first if pending ops require it) → `Sign` as the last incremental revision (redact → rewrite → sign, ADR 0007 §4). `Sign` is a barrier op: undo history before it is kept for viewing but not undoable across the signature. Keys never leave the device; on the web only the timestamp request (a hash) goes out, and the network indicator counts it ("1 request · to timestamp.example.org only").

**Accessibility.** Identity cards are labelled radios (name + issuer + expiry read in order); the preview has a text alternative; the timestamp checkbox carries the focus ring on the board. Errors use `aria-describedby`/`aria-invalid`. Escape cancels before signing starts.

**Platform differences.** Web (G02-timestamp-web): identities are imported PKCS#12 files kept on the device (WebCrypto non-extractable keys). A timestamp server that browsers can't reach needs lamina's TSA proxy (E10e), which is opt-in: until it is allowed, **Add a trusted timestamp** stays off with "Off until you allow the proxy below. Without it, the signing time comes from this device’s clock." Under it a consent block (white, `control-border`) titled "This server needs lamina’s proxy" says "Only a hash is sent, via lamina’s proxy. Nothing is sent until you allow it.", lists What is sent (a hash of the signature, never the file) · Where to (lamina’s timestamp proxy, then timestamp.example.org) · How long it stays (the proxy passes it on and doesn’t log it) · What it is used for, and offers **Allow and add timestamp**. Allowing ticks the checkbox and is remembered in Settings › Privacy (where it can be turned off); each request is counted by the network indicator ("1 request · to lamina’s timestamp proxy"). macOS: Keychain; Windows: certificate store (label "Windows certificate store"); Linux: NSS / PKCS#11. Phone: the dialog is a large-detent sheet (reached from G11), web identities only.

**Artboards.** `G02-sign-certificate.dc.html`, `G02-timestamp-web.dc.html` (640 × 760, the web dialog before the proxy is allowed).

**Todos.** [E10j](../../todos/ready/E10j-signature-ui.md), [E10i](../../todos/ready/E10i-platform-signers.md), [E10e](../../todos/ready/E10e-timestamps-and-tsa-proxy.md).

## G03 Certify and lock

**Purpose.** Certify the document (DocMDP on the first signature), choose which later changes stay allowed, and lock fields when signing (FieldMDP) (E10f).

**Entry points.** **Certify instead…** in G02; palette "Certify document" (`sign.certify`); Fill & Sign toolbar overflow (…). No shortcut.

**Layout.** Desktop window; dialog (max 560) over the dimmed Fill & Sign shell, same family as G02. The note under the options reads "Certification must be the first signature. This file has none yet, so you can certify it. After saving, edits this level doesn’t allow show a warning before you save, and you can save a copy instead."

**Content and controls.**
- Title "Certify document" and its one sentence.
- **Certify as** summary row (identity chosen in G02) + **Change…**.
- **Allowed after certifying** radios: No changes (P1) · Filling in forms and signing (P2, default and recommended when other signature fields are empty) · Filling in forms, signing and comments (P3), each with a one-line consequence.
- **Lock fields when you sign**: Don’t lock fields · Lock every field · Lock only these fields (a checklist of fields with their page).
- **Add a trusted timestamp (only a hash is sent)**.
- Note "Certification must be the first signature." with the current fact ("This file has none yet").
- **Sign without certifying** (back to G02), **Cancel**, **Certify and save** (primary).

**States.** Already signed: the dialog opens with the note in `warning` ("This file already has 1 signature, so it can’t be certified. Sign it instead.") and Certify and save is disabled with that reason. No fields: the lock section is hidden. After saving, edits the chosen level forbids raise the constraint banner (§7.13) on the first such edit.

**Behaviour.** One `Sign { certify: Some(P), lock }` op, saved like G02. The level is recorded and shown in G04 ("Certified · allows form filling, signing and comments").

**Accessibility.** Two fieldsets with legends; the field checklist is a labelled group; the selected radio shows the focus ring on the board.

**Platform differences.** Identical on web and desktop apart from identity sources (see G02). Not offered on the phone (opens "Certify on a larger screen" help).

**Artboards.** `G03-certify.dc.html`.

**Todos.** [E10f](../../todos/ready/E10f-certification-and-locking.md).

## G04 Signature validation

**Purpose.** On open, validate every signature on the device and show the overall result as a status-bar badge; the Signatures panel explains each signature in plain words; the certificate details dialog shows the chain (E10g, with the panel UI from E10j).

**Entry points.** Automatic on open (badge); clicking the badge or "Details" opens the rail's **Signatures** tab (`signatures.show-panel`); clicking a signature on the page selects it and opens the inspector; palette "Show signatures".

**Layout.** Web app shell (drawn first; inventory W, D). Read mode (nothing here changes the file; "Trust on this device…" is a device setting). Rail 220 px with Pages · Outline · Signatures (§8.10; the Signatures tab appears only when the file has signatures, and the PDF's bookmarks are in Outline). Canvas on page 24 with both signature appearances, the selected one outlined. Inspector "Signature". Status bar: network indicator · signature badge + "2 signatures · Details" · page label.

**Content and controls.**
- *Status-bar badge*: the worst result across signatures, with a word: "Signature valid" (positive), "Signed, changed after signing" (warning), "1 signer not trusted" / "Signature not trusted" (danger), "Changed after signing" (danger, see G05), "Checking signatures…" (neutral).
- *Signatures panel*: heading "2 signatures" + "3 versions in file"; one row per signature (name, version, badge, one-sentence explanation); footer "Checked on this device when the file opened · time", the revocation source, **Check again**.
- *Inspector* (`dl`): Signed by · Signer’s identity (trusted/not, issuer) · What she signed (unchanged, version, pages) · Changes after signing (each, and whether allowed) · Signing time (trusted timestamp or the signer’s clock) · Certification level · Revocation. Actions **Compare with signed version** (G05) and **Show certificate…**.
- *Certificate details dialog* (G04-certificate-viewer): status notice (danger for an untrusted self-signed certificate: "Not from an authority you trust… lamina can confirm the file hasn’t changed since it was signed, but not who signed it"); **Chain of trust** tree; Issued to, Issued by, Valid, Allowed uses, Key, Fingerprint (monospace); advice to check the fingerprint by another route; **Export certificate…**, **Trust on this device…** (asks for confirmation, then re-validates), **Done**.

**States.** Checking (badge neutral, rows with skeleton badges); valid; signed and changed after signing (allowed changes, warning); signer not trusted (danger); invalid (integrity broken, danger "Signature invalid", explanation "The signed content has changed"); revocation not reachable ("Revocation couldn’t be checked: offline", neutral note, not a failure); expired certificate valid at signing time (positive with note). No signatures: the tab is absent and the status bar shows nothing.

**Behaviour.** Validation is read-only and runs in the worker after first paint. Online revocation checks are opt-in on the web (each request shows in the network indicator); desktop uses the OS trust store. Trusting a certificate is stored per device and is reversible in Settings.

**Accessibility.** The badge is a button whose name includes the result ("1 signer not trusted, 2 signatures, show details"); badges always pair colour with words and an icon (shield-check, warning triangle, shield-cross); rows are buttons with `aria-current` for the selected one; the chain is a `role="tree"`; result changes after "Check again" are announced politely.

**Platform differences.** Desktop: same panel; trust anchors from the OS store, "Trust on this device…" becomes "Add to trusted certificates…". Phone: the badge sits in More and in the page header subtitle ("Signed · 1 signer not trusted"); the panel is a sheet at the medium detent; the inspector becomes the sheet's detail view.

**Artboards.** `G04-signatures-panel.dc.html`, `G04-certificate-viewer.dc.html`.

**Todos.** [E10g](../../todos/ready/E10g-signature-validation.md).

## G05 Revision diff and attack warnings

**Purpose.** Show, object by object and side by side, what changed after a signature, classify each change as allowed or not, and warn plainly when a change is not allowed or matches a known attack (shadow, incremental-saving, overlay) (E10h).

**Entry points.** **Compare with signed version** in G04 (`signatures.compare`, which opens the compare view of [§8.12](design-language.md#812-compare-proposed) with the signed-revision preset); the danger banner that appears on open when validation finds a disallowed change; palette "Compare with signed version" (`signatures.compare`).

**Layout.** Desktop window (drawn first; inventory D). A file received from someone else (`Q3-board-report-signed.pdf`, a tampered copy of the G04 file). Under the top bar, a **danger banner** (`role="alert"`, `danger-tint`): "Changed after signing. Page 3 now shows different content from what Amara Okafor signed. Her signature covers the signed version only." + **Open signed version**. The compare view is Convert and produce X04's (canonical, §8.12) opened with a preset: its top bar ("Compare", **Older** picker "Signed version 1 · Amara Okafor", Swap, **Newer** picker "Q3-board-report-signed.pdf · current (version 3)", Side by side / Overlay, Previous change · "1 of 3" · Next change, zoom, Find, View, **Close compare**, More) and its "Show" toolbar (Text · Images and drawings · Comments · Signatures toggles with counts, Scroll together, and the note "Preset: changes after Amara Okafor’s signature · object by object"). The window title is "Compare · Q3-board-report-signed.pdf". Rail (260 px, as X04) "Changes after signing" grouped by version. Canvas: the two versions of page 3 side by side, with the diff marks of §2.7 (signed value in `danger-tint` struck through, current value in `positive-tint` underlined, changed bars in dashed outlines, the current change with the 2 px `accent` outline) and a danger "1 · Changed after signing" tag.

**Content and controls.** Each change row: title, badge ("Not allowed" danger / "Allowed" positive), what changed in plain words ("The chart now shows West 97; the signed version shows 197"), and for attacks the pattern name ("Pattern: shadow attack (content swapped without touching the signed bytes)"). Rail footer explains what "Allowed" means for this certification. **Open signed version** opens that revision read-only in a new tab/window titled "… (signed version 1)"; **Export signed version…** sits in its More menu.

**States.** Only allowed changes: no banner, warning badge in G04, compare still available. LTV-only revisions (DSS, document timestamps) are listed as "Validation data added · Allowed" and never trigger a warning. Overlay over content (a signature widget or annotation drawn over signed text): row "Something now covers signed content" with the overlaid area outlined. Cannot compare (revision damaged): neutral notice "lamina can’t rebuild version 1 of this file" and the change list stays.

**Behaviour.** Read-only; nothing in the file changes. Previous/Next change scroll both pages in sync and move the rail selection. Overlay mode shows the current version at 50% over the signed one with a slider (keyboard: arrows).

**Accessibility.** The banner is `role="alert"` once per open; each change has a text description, so the visual diff is never the only carrier; Previous/Next change are reachable from the keyboard; the comparison `main` is labelled "Comparison of page 3".

**Platform differences.** Web: same view; the second window becomes a new browser tab. Phone: no side-by-side; the changes list is a sheet and "Open signed version" shows that version full screen.

**Artboards.** `G05-revision-diff.dc.html`.

**Todos.** [E10h](../../todos/ready/E10h-revision-diff-and-attacks.md).

## G06 Protect

**Purpose.** Encrypt with AES-256 and set an open password, a permissions password and permission limits; remove or change protection with authorisation; protect for certificate holders instead of a password (experimental) (E11b, E11c, E11e).

**Entry points.** **Protect** on the Edit toolbar (`security.protect`; visible, not in Tools ▾, per design language decision 3); palette "Protect document…", "Remove protection…" (`security.remove-protection`); the security item at the right of the status bar ("Protected · AES-256 · …", after the page label) opens G06-remove-security's summary. From Read mode these switch to Edit first (§8.11). No shortcuts.

**Layout.** Desktop window; dialogs (max 560, remove: 480) over the dimmed Edit shell with the Edit toolbar of §8.1 (Select · Edit text · Add text · Image · Shape ▾ · Link │ Page marks ▾ · Replace… │ Redact · Protect · Tools ▾) and Protect pressed. G06-remove-security draws the status-bar security item (Lock icon, "Protected · AES-256 · opened with the open password") at the right, after "Page 1 of 24". G06-protect's sample file is `Q3-board-report.pdf` with Amara Okafor's and Daniel Reyes's signatures (status bar "Signature valid · 2 signatures"); `Q3-board-report-signed.pdf` is only the tampered copy of G04–G05.

**Content and controls.**
- *Protect document, Password tab* (G06-protect): **Require a password to open** + Open password and Type it again (masked; help "18 characters. lamina can’t recover it: store it somewhere safe."); **Limit printing, copying and editing** + Permissions password, **Printing** select (High quality / Low resolution only / Not allowed), allowed actions (Copy text and images · Fill in forms and sign · Add comments · Insert, rotate or delete pages), and the honest note "Screen readers can always read the text. Most apps respect these limits, but they are not a lock…"; **Also encrypt the title, author and other document information**. On a signed file a warning notice (`role="note"`, not an alert, so it isn't announced twice when the dialog opens) names the signatures that will be removed. Actions: **Save a protected copy…** (keeps the signed original), **Cancel**, **Remove signatures and protect** (danger filled; on an unsigned file: **Protect and save**, primary).
- *Remove protection* (G06-remove-security): a summary `dl` (Encryption, To open, Limits, Information), the reason the permissions password is needed, the **Permissions password** field, the rewrite note, **Change passwords or limits…** (reopens Protect pre-filled), **Cancel**, **Remove protection**.
- *Protect document, Certificates tab* (G06-public-key): "Experimental" badge on the tab; **People who can open it** list (you are always included and can’t be removed; others with an access select: Full access / Read and print / Read only, and a Remove icon button); **Add from a file (.cer, .p7b)…**, **Add from Keychain…**; a warning note on compatibility ("Files protected this way open in apps that support certificate security, including the lamina desktop app…"); **Protect and save**.

**States.** Passwords don’t match: error under "Type it again". Wrong permissions password: drawn (danger border, `aria-invalid`, message "That password doesn’t match. Check with whoever protected the file."). Opened with the owner password already: Remove protection needs no field. Certificates tab with only you: help "Add at least one other person, or use a password". Saving: "Encrypting… 62%" progress bar.

**Behaviour.** `SetEncryption` / `RemoveEncryption` / public-key write are barrier ops that require a full rewrite (no earlier revision survives); one undo step until saved. Passwords stay in the worker and never leave the device. After saving, the status bar shows the security item.

**Accessibility.** Show/hide password toggles are buttons with `aria-pressed` (not drawn; at the right of each field); checkboxes and radios are real inputs in labelled groups; the focus ring is on the open password (G06-protect), the permissions password (remove) and an access select (certificates).

**Platform differences.** Web: identical; the Certificates tab says certificates come from imported files, and recipients that need RSA PKCS#1 v1.5 can open the file only after the explicit PKCS#12 opt-in (E11e). Windows: "Add from the certificate store…"; Linux: "Add from NSS…". Phone: Protect and Remove are large-detent sheets; the Certificates tab is desktop and web only.

**Artboards.** `G06-protect.dc.html`, `G06-remove-security.dc.html`, `G06-public-key.dc.html`.

**Todos.** [E11b](../../todos/ready/E11b-encrypt-on-write.md), [E11c](../../todos/ready/E11c-remove-security.md), [E11e](../../todos/ready/E11e-public-key-security.md).

## G07 Sanitise

**Purpose.** Remove hidden information by category (metadata, comments, attachments, scripts and actions, hidden layers and off-page content, earlier versions), showing what was found before and a report of every removed item after (E11d).

**Entry points.** More menu and palette "Sanitise document…" (`security.sanitise`; from Read mode it switches to Edit first, §8.11); offered from Share/Export when the file has scripts or attachments ("This file contains 1 attachment and 1 script. Sanitise first?"). No shortcut.

**Layout.** Desktop window, Edit mode (Edit toolbar, no tool pressed). The dialog (max 560) over the dimmed shell; after removal, the report in the wide docked panel (360 px, §6.1) beside the page, a toast "15 items removed · Undo", and Save in the top bar.

**Content and controls.**
- *Dialog*: summary sentence with the total found; **Preset** select (Custom, Before sharing outside the organisation, Everything); a checklist of categories, each with what was found in plain words, a count, and an expand button listing the items; "Earlier versions" is always on (a checked, disabled box with its reason as the description). Drawn: Comments and markup unticked ("kept in this preset"), the others ticked, so the button reads "Remove 15 items". Note on rewrite and undo. **Cancel**, **Remove 15 items** (the count follows the selection).
- *Report panel*: positive summary notice with the post-removal scan result; one section per category with each removed item (check icon) and a "Kept" section for unticked categories; **Export report…** (JSON or text), **Copy as text**.

**States.** Scanning: the checklist shows skeleton rows and "Looking through the file…". Nothing found: empty state "Nothing hidden to remove" with **Close**. Dynamic XFA form: the Scripts row warns "This form is built with XFA; removing its scripts means it won’t display as before" (warning). Signed file: the signature warning notice from G06 appears above the actions and the button becomes "Remove signatures and sanitise".

**Behaviour.** Applies `Sanitise { categories }` (requires rewrite) as one undo step; the report is generated from the ops' results and is kept until the file is closed. Saving writes a single revision.

**Accessibility.** The checklist is a fieldset; counts are part of each checkbox's description; expand buttons use `aria-expanded`; the report is an `aside` with headings per category.

**Platform differences.** Web identical. Phone: dialog and report become sheets (large detent).

**Artboards.** `G07-sanitise.dc.html`, `G07-sanitise-report.dc.html`.

**Todos.** [E11d](../../todos/ready/E11d-sanitiser.md).

## G08 Redact mode: mark and review

**Purpose.** Mark text and areas for redaction as Redact annotations, review every mark (accept or reject), and preview the page as it will look after applying; nothing is removed in this step (E12b).

**Entry points.** **Redact** on the Edit toolbar (`redact.open`; visible, not in Tools ▾); palette "Redact…"; selection mini toolbar More → "Mark for redaction" (`redact.mark-selection`). No single-key shortcut (redaction is deliberate).

**Layout.** Desktop window (drawn first; inventory D). Top bar Edit active, "12 unsaved changes", Save. The **Redact toolbar** (§8.1) replaces the Edit toolbar: "Redact" title · **Mark text** · **Mark area** · **Find sensitive…** (an action that opens the Find sensitive tab, so it keeps its `…` and is not a toggle) │ **Preview result** (toggle) · "12 marks · nothing removed yet" · **Apply redactions…** (secondary) · **Done**. Rail 280 px: Pages · Outline · **Review** visible, Find sensitive in the More tabs menu (§8.10). Canvas page 23. Inspector "Redaction mark". Status bar: "Saving now keeps the marks only; nothing is removed until you apply".

**Content and controls.**
- *Marks on the page*: `danger-tint` fill with a 1.5 px `danger` outline (accepted), dashed outline (to review); the selected mark adds the 2 px `accent` ring. Text under a mark stays readable until preview.
- *Review list* ("Marked for redaction", "9 accepted · 2 to review · 1 rejected", **Accept all**, **Next to review**): grouped by page; each row is the marked text or area description and kind ("Text · email", "Area · image"), plus **Accept** and **Reject** toggle buttons (rejected rows are struck through).
- *Inspector*: quoted text; **Text shown in the box** (overlay text, optional); **Box colour** (Black, White); **Also found** (where the same text appears in other layers, removed on apply); note "Marked, not removed."; **Reject**, **Delete mark** (danger ghost).
- *Preview result* (tweak `preview` on the artboard): marks render as solid boxes and the text under them is hidden, exactly as Apply would draw them; the page is read-only while previewing.

*Drawn sample* (G08-redact-mode, after G09): 12 marks. Page 2: "R. Osei, case lead" (accepted). Page 23: the card number (selected, with the inspector), four tenant emails, the NI number, the postcode LS9 7QT in R. Osei's comment (marked from G09) and an area over the bathroom-ceiling photo (accepted); the four phone numbers are not marked (left unticked in G09: this copy keeps them for the surveyors); the tenant names Joan Whitfield and Samir Haddad (to review, dashed); "East region" (rejected: dashed `muted` outline on the page, struck through in the list). The selected list row uses `accent-tint` with an inset `accent` edge; the board's focus ring is on **Next to review**.

**States.** No marks: the Review tab shows the empty state "Nothing marked yet" with "Select text and choose Mark text, or use Find sensitive"; Apply is disabled ("Mark something first"). **Apply applies accepted marks only**: while marks are to review it stays available and its confirm (G10) says how many are left as they are; with marks but none accepted it is disabled with the reason "Accept at least one mark first" (`aria-describedby`), next to **Accept all**. Area marks over images show the pixel area that will be blanked. A mark that touches a signature appearance shows a warning row.

**Behaviour.** Mark text works on the current selection or as a drag tool; Mark area draws a rectangle (keyboard alternative: "Mark area…" asks for the region with arrow keys from a starting box). Each mark is one `MarkRedaction` op (class Annotation, saves incrementally; a violation on certified P1/P2 files, which raises the constraint banner). Accept/Reject is review state stored with the mark; Reject keeps it visible but excluded from Apply; Delete mark removes it.

**Accessibility.** Accept/Reject buttons have names including the mark text and `aria-pressed`; the list is navigable with arrows and Enter selects the mark on the page; marks are focusable on the page with the same names; colour is never the only signal (dashed vs solid, state words in the row).

**Platform differences.** Web identical. Phone: Redact is not offered (inventory has no phone screen); the More menu shows "Redact on a larger screen".

**Artboards.** `G08-redact-mode.dc.html`.

**Todos.** [E12b](../../todos/ready/E12b-mark-and-review.md).

## G09 Find sensitive content

**Purpose.** Find redaction candidates with built-in patterns (emails, phone numbers, card numbers with a check digit, NI numbers, bank accounts and IBANs, postcodes), words or a regular expression, across page text, comments, form fields, document information and bookmarks (and OCR text when present), then mark all or selected (E12e).

**Entry points.** Redact toolbar **Find sensitive…** (`redact.find-sensitive`); the rail's Find sensitive tab; palette "Find sensitive content".

**Layout.** Desktop window. Redact toolbar; the rail (280 px: Pages · Outline · **Find sensitive** visible, Review in More tabs) holds the search; canvas page 23 with every match in the find marks of [§2.6](design-language.md#26-find-marks-proposed) (`find-match` with a 1 px `warning` bottom edge, the current one `find-current` with the 2 px `accent` outline); status bar "Match 1 of 7".

**Content and controls.** **Words or pattern** input + **Regular expression** checkbox; **Patterns (UK)** checklist with counts (locale follows Settings); **Look in** checklist ("Text recognised in scans" disabled with "This file has no recognised text. Run Recognise text first."); **7 matches** with **Select all**, rows of match text + where it was found, each with a checkbox; the hint "Patterns don’t find names. Type a name above, or use Mark text."; a note "Unticked matches are not marked; they stay in the file."; footer "3 of 7 selected" + **Mark 3 selected** (primary).

*Drawn sample* (G09-sensitive-search): the four tenant emails are already marked (shown as accepted marks on the page), so **Email addresses** is unticked with "4 already marked". The 7 matches are the four phone numbers in the table (the first is current, ringed, and its row has `accent-tint`), the card number, the NI number and a postcode in R. Osei's comment on page 23 (listed with its location, not visible on the page). The phone numbers are unticked on purpose: this copy goes to the surveyors, who need them; the other three are ticked. The story continues in G08 (these marks plus manual ones) and G10 (the phone numbers stay readable on the redacted page). Bank accounts and IBANs: 0. Look in: Page text, Comments, Form fields, Document information, Bookmarks (all ticked).

**States.** No matches: "No matches for these patterns" in the results area. Invalid regex: error under the input ("This pattern isn’t valid: missing closing bracket"). Searching a large file: indeterminate progress under Look in, results stream in. After marking: toast "3 marks added · Undo" and the Review tab shows them as "to review".

**Behaviour.** Search runs in the worker; selecting a result scrolls the page to it. Mark produces one `MarkRedaction` per selected match, as a single undo step. Matches outside page text (comments, keywords) are listed so the user knows Apply will remove them there too.

**Accessibility.** Result checkboxes are labelled with text + location; results count is announced politely; Enter on a row moves to the match; the Mark button carries the focus ring on the board.

**Platform differences.** Web identical. Phone: not offered.

**Artboards.** `G09-sensitive-search.dc.html`.

**Todos.** [E12e](../../todos/ready/E12e-sensitive-search.md).

## G10 Apply redactions and verification report

**Purpose.** Confirm what applying will do, apply (remove glyphs, image pixels and paths under the marks, scrub every other layer, rewrite the file with no earlier versions), then prove it with a per-layer verification report; saving stays blocked while any check finds a leak (E12c, E12d).

**Entry points.** Redact toolbar **Apply redactions…** (`redact.apply`); palette "Apply redactions…". No shortcut.

**Layout.** Desktop window. Confirmation: an `alertdialog` (max 560, §7.18) over the dimmed Redact shell. Report: a danger strip under the top bar, the Redact toolbar ("9 redactions applied · undo with ⌘Z until you save", Apply hidden), the redacted page, and a docked **Verification report** panel (360 px). Status bar badge "Leak found".

**Content and controls.**
- *Confirm* (G10-apply-confirm): title "Apply 9 accepted redactions?"; one-sentence consequence; a list: accepted marks and pages ("Only accepted marks are applied."), then a `warning` "2 to review" badge with "2 marks to review are left as they are: Joan Whitfield, Samir Haddad. So is the rejected mark “East region”." and **Review them first**, other layers where the same text is removed, the full rewrite dropping earlier versions, **signatures that will be removed, by name** (tweak `signed`), and that the report follows and saving needs every check to pass; "You can undo this in one step until you save."; **Cancel** (initial focus), **Apply 9 redactions** (danger filled).
- *Report* (G10-verification-report): what was checked in one sentence ("searched the rewritten file for each redacted text in every layer, rendered each changed page and compared it with the marks"); one row per check with a badge: Page text · Text positions · How the pages look · Images and drawings · Comments and form fields · Document information · Bookmarks, links and page labels · Accessibility tags and alt text · Earlier versions · Attachments; a failing row expands with the cause and the fix (**Remove attachment**, **Show it**); **Check again**, **Export report…**.
- *Save blocked*: Save in the top bar is disabled with `aria-describedby` pointing at the banner text "Saving is blocked: 1 leak found. The attachment case-notes.xlsx still contains “j.whitfield@example.org”." with **Remove attachment**.

*Drawn sample*: the confirm dialog lists 9 accepted marks on pages 2 and 23; the 2 marks to review and the rejected one are left as they are; other places the text is removed (2 comments, accessibility tags, document information); the rewrite dropping 3 earlier versions; with `signed` on, a `warning-tint` notice "Removes 1 signature. Amara Okafor (Chair), signed with a certificate on 27 Oct 2026…". The report shows 9 rows "Passed" (each with how it was checked, e.g. "Rendered pages 2 and 23 and compared each box with its mark.") and the Attachments row expanded with "Leak found": "case-notes.xlsx (48 KB) still contains “j.whitfield@example.org”…" and the fix. The page shows solid `#1F1D1A` boxes where the accepted marks were; the tenant names (still to review) and the phone numbers (never marked) stay readable. Focus ring on the Attachments row's disclosure button. Top bar: "Redactions applied · not saved", Save `aria-disabled` (kept focusable so its reason can be read) in the *blocked primary* style (`hairline` fill and edge, `text-3` label, 7.09:1; see the proposal below).

**States.** Applying/checking: the panel shows each check with a neutral "Checking…" badge and an overall progress bar. All passed: positive notice "All 10 checks passed. Nothing you redacted can be found in the file." and Save enabled; saving writes one revision (then signing, if pending, adds a second, ADR 0007 §4). Leak found: drawn. An image that can’t be decoded: the check fails with "lamina can’t edit this image; remove it or mark the whole image".

**Behaviour.** `ApplyRedactions` is a barrier op requiring a rewrite; one undo step restores the marks until saving. Fix actions (Remove attachment) add their own op and re-run the checks automatically. The report is kept with the session and exportable as JSON; it is not written into the PDF.

**Accessibility.** The confirm dialog is `role="alertdialog"` with focus on Cancel; the banner is `role="alert"`; each check row reads name, result and how it was checked; redaction boxes on the page are `role="img"` named "Redacted"; the disabled Save explains itself through `aria-describedby`.

**Platform differences.** Web identical; very large files may suggest the desktop app before applying (web `Limits`). Phone: not offered.

**Artboards.** `G10-apply-confirm.dc.html`, `G10-verification-report.dc.html`.

**Todos.** [E12c](../../todos/ready/E12c-apply-content-removal.md), [E12d](../../todos/ready/E12d-scrub-and-rewrite.md).

## G11 Phone: sign a document

**Purpose.** On the phone, choose or make a signature image (or start a certificate signature), place it on the page and save (E10j).

**Entry points.** Tapping an empty signature field; More → **Sign** (`sign.open-sheet`); the bottom bar's Annotate entry when the file has signature fields.

**Layout.** Phone shell (390×844). *Sign sheet* (G11-phone-sign-sheet) at the medium detent over page 24, the target field outlined above it: header "Sign · Chair field" + close; **Add a signature image** with its sentence, two saved signatures as 64 px tiles, **New signature: draw, type or photo** (opens G01 phone); divider; **Sign with a certificate** with its sentence and **Choose a certificate…** ("1 on this device", opens G02 as a large-detent sheet). *Placing* (G11-phone-sign-place): the image fitted to the field with the selection outline and two `accent` handle dots; the top bar subtitle "1 unsaved change" and a **Save** text button replace the overflow; a hint "Drag to move, pinch to resize, or use the buttons below. This is a picture of your signature, not a digital signature."; a bottom action bar (§7.15: 76 px, 52 px buttons) **Move…** · **Smaller** · **Larger** · **Delete** · **Done**.

**Content and controls.**
- *Sign sheet*: grabber (**Resize sheet**), title "Sign · Chair field", **Close**. Section **Add a signature image** (icon, title, "Puts a picture of your signature on the page; it doesn’t prove who signed."), two saved-signature tiles ("Use saved signature: drawn", "Use saved signature: typed"), **New signature: draw, type or photo** (`sign.add-signature-image`, opens G01 phone). Divider. Section **Sign with a certificate** ("Proves who signed and shows any change made after signing."), **Choose a certificate…** with "1 on this device" (`sign.with-certificate`, opens G02 as a sheet). The page behind keeps the Chair field in the target style (1 px `accent` outline on `accent-tint`), so it isn't mistaken for the focus ring. Focus ring on Choose a certificate….
- *Placing*: top bar subtitle "1 unsaved change", **Save** text button (44 px) in place of More; the placed image with a 1.5 px `accent` selection outline and two 12 px `accent` handle dots (top-left, bottom-right); the hint note under the page; action bar (`role="toolbar"`, "Signature image"): **Move…** · **Smaller** · **Larger** · **Delete** (`danger`) · **Done** (`accent`, focus ring). The placed image is a button named "… Selected" (no `aria-pressed`). Tweak `saved` on the artboard: subtitle back to "Page 24 · Board approval", More returns, the reader bottom bar returns and a "Saved" toast (§7.20: radius 6, padding `8px 14px`, 13 px) shows above it.

**States.** No saved signatures: the tiles are replaced by **New signature** as the first, larger button. Saved: toast "Saved" above the bottom bar, the bottom bar returns. Certificate signing finished: badge in the header subtitle ("Signed · Signature valid").

**Behaviour.** Choosing a saved signature places it immediately in the field (one op, undoable). Move… opens a small sheet with arrow buttons and "Move to field…" (the non-drag alternative, WCAG 2.5.7). Save is the explicit save (incremental).

**Accessibility.** All targets ≥ 44 px; saved tiles are named ("Use saved signature: drawn"); the placed image is announced as selected; Done carries the focus ring on the board.

**Platform differences.** The phone has no secondary toolbar, so Highlight fields and Form data ▾ (Fill & Sign, §8.1) live in the phone's Fill & Sign touch bar (Forms P04), not in this sheet. Tablet: desktop layout with touch rules (action bar instead of handles-only). Native apps (later, ADR 0025) would use the system keychain for certificates.

**Artboards.** `G11-phone-sign-sheet.dc.html`, `G11-phone-sign-place.dc.html`.

**Todos.** [E10j](../../todos/ready/E10j-signature-ui.md).

## New components

Merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table.

## Proposed design-language additions

Earlier proposals were merged into the [design language](design-language.md#decisions-on-area-proposals) on 2026-10-29; see its decisions table. New since then, from the final review:

- **Mark area icon** (closes the open point in decision 15: it looked like Focus mode): a dashed rectangle with a square corner handle, `<rect x="4.5" y="5.5" width="12" height="10" stroke-dasharray="2 2"></rect><rect x="15" y="14" width="4.5" height="4.5"></rect>`.
- **Target style for a page object a dialog or sheet acts on** (the field being signed behind G01's menu, G02, G03 and G11's sheet): 1 px `accent` outline, offset 2, on an `accent-tint` fill. It is not the focus ring, so a board still shows one 2 px ring.
- **Blocked primary** (Save while a report blocks saving, G10): `hairline` fill and edge with a `text-3` label (7.09:1 on `hairline`; `muted` would be 4.36:1), `aria-disabled="true"`, kept focusable, with `aria-describedby` pointing at the strip that says why. Unlike the §7.0 disabled primary, its label stays readable because its reason matters.
- **Redact Apply covers accepted marks only** (§8.1 Redact row and §8.5): the confirm repeats the count of accepted marks on its button and lists the marks to review and rejected ones as "left as they are".
