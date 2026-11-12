# E1e — Save, Save as, Download copy and the SaveConstraint banner

| Field | Value |
| --- | --- |
| ID | `E1e` |
| Status | `done` |
| Plan | [E1 editing foundation §7](../../plans/editor/e1-editing-foundation.md#7-saving-and-the-save-constraint) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1c](./E1c-metadata-ops.md), [E1d](./E1d-edit-session-undo-ui.md) |
| Dependency notes | [E1c](../done/E1c-metadata-ops.md) (`SaveOptions { metadata_date, instance_id }`, `ConformanceWarning`, `SaveReport::metadata_not_updated`), [E1d](../done/E1d-edit-session-undo-ui.md). The desktop's save in place is [E1i](../ready/E1i-desktop-save-in-place.md), after D3b; the constraint table is [E1 plan §7](../../plans/editor/e1-editing-foundation.md#7-saving-and-the-save-constraint) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1e-save-and-constraint-banner` |
| PR title | `feat(app): save through the platform adapter with the save-constraint banner` |
| Size | L (600–1,500) |
| PR | [#444](https://github.com/laudebugs/laminapdf/pull/444) |

## Goal

Let users save edits safely on the web: Save (incremental by default), Save as, Save as optimised (full rewrite) and Download copy through the `Platform` adapter, with a banner that shows the document's `SaveConstraint` (Free, AppendOnly, RequiresRewrite, Conflict), names every signature a save would invalidate, and asks before any override.

## Context

- **Design (Paper):** [O02 Save, Save as and Download a copy](../../design/paper/edit-session-organise.md#o02-save-save-as-and-download-a-copy). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [ADR 0007](../../decisions/0007-save-planner.md): planning vs writing, `SaveConstraint.kind()`, override, sinks and rebase
- [ADR 0005](../../decisions/0005-document-model.md) §3: rebase after save and `SavedNotRebased`
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) "Web vs desktop"; [full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 2

## Scope

**In scope**

- Save commands: `Save`, `Save as…`, `Save as optimised` (full rewrite), `Download a copy`
- Web: File System Access API (`showSaveFilePicker`, `createWritable`) where available; streamed `WriteSink` into it; otherwise download via Blob URL; OPFS spool for full rewrites (readable sink for rebase)
- Desktop: in-place save and outside changes → [E1i](../ready/E1i-desktop-save-in-place.md)
- `SaveConstraint` banner component: kind, signatures (field, signer, kind), rewrite reasons, violations; explanation texts; confirm dialog naming each signature before `invalidate_signatures: true`
- Show `ConformanceWarning`s from E1c before saving, and say when save-time metadata was left unchanged to keep a certification valid (`metadata_not_updated`)
- Pass save-time inputs (`metadata_date`, `instance_id`) from the shell (clock at the edge)

**Out of scope** (and which todo covers it)

- Capability-matrix page and metadata UI → [E1f](../ongoing/E1f-metadata-ui-capability-page.md)
- Optimisation options beyond a plain full rewrite → [E16b](../ready/E16b-structural-optimiser.md)

**Editor facts**

- **Crates and packages:** `laminapdf-save` (sinks), `laminapdf-wasm` (streaming sink adapter), `@laminapdf/ui` (banner, dialogs), `apps/web`'s platform adapter
- **`SaveConstraint`:** implements the UI for all four kinds: `Free` → incremental default; `AppendOnly` → incremental only, full rewrite blocked; `RequiresRewrite` → full rewrite, old revisions dropped (said in the dialog); `Conflict` → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** web here: in place only where the File System Access API exists, otherwise Save becomes Download and the document rebases onto what was downloaded. The desktop's save in place is E1i
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (infrastructure); unblocks `write`/`round-trip` claims for every later op in web and desktop columns

## Tasks

- [x] Extend the `Platform` adapter (ADR 0010) with `saveInPlace`, `saveAs`, `download`, `canWriteInPlace` already defined; add `readableSpool()` for rebase
- [x] Web: streaming writable sink in the worker; fallback download path; OPFS spool; tests for Chromium (FSA) and Firefox/WebKit (download)
- [x] Banner and dialogs in `@laminapdf/ui` with copy reviewed against ADR 0007 wording
- [x] Rebase handling: after a full rewrite to a non-readable sink, call `rebase` with the spooled bytes; block ops while `SavedNotRebased`
- [x] Every save passes the save time (`metadataDate`, from the clock) and a fresh `instanceId` (a random UUID), so the file's modification date and XMP instance ID change ([E1c](../done/E1c-metadata-ops.md): a save without them stamps nothing)
- [x] Error paths: disk full, permission denied, FSA permission revoked → document stays dirty, overlay untouched
- [x] The edit session's `plan(options)` in the document worker (E1 plan §5), which E1d left for this todo since the SDK had no save plan yet; mark the session's saved position after each save (`markSaved`)
- [x] The close confirmation's Save button (Paper O03), which E1d shipped with Cancel and Close without saving only
- [x] The History tab's footer sentence on Save ("History is kept after Save and cleared by Save as optimised", O01) once saving exists
- [x] Undo and Redo's tooltips naming the step with the keys (O01: "accessible name and tooltip name the step"), which E1d left out to keep the startup script in its budget: load the tooltip with the editor's parts after open

## Acceptance criteria

- [x] e2e: edit → Save → reopen in the app and in pdf.js shows the change
- [x] On a signed fixture the banner shows AppendOnly with the signer; "Save as optimised" is blocked; a DocMDP-violating op shows the named signature and requires confirmation
- [x] A failed write leaves the document dirty and the original file intact (a refused File System Access write in Chromium)
- [x] Web: Chromium saves in place via FSA; Firefox and WebKit download a copy

## Post-merge checks

None.

## Tests

- Playwright: `apps/web/e2e/save.e2e.test.ts` (FSA mocked in Chromium, download path elsewhere)
- Signed fixtures for banner states

## Docs to update

- User-facing help page on saving and signatures
- ADR 0010 if the adapter gains methods
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- FSA API permission prompts differ across Chromium versions; keep the download fallback always available
- Append-in-place on network drives/cloud-synced folders may be non-atomic; recommend Save as for those (detect where possible)

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Saving is a copy, then its adoption.** The worker writes the file as the save would (`copyEdits`, over the new `Save::save_copy`), the page writes it through the `Platform`, and only then does the worker make the copy the document's saved file (`adoptEdits`, over the new `Save::adopt_copy`). A failed or cancelled write leaves every edit unsaved. `SaveReport::history_cleared` says whether the save emptied the undo history, so the session's history follows the model's. [ADR 0007 §10](../../decisions/0007-save-planner.md) records the three.
- **The redraw draws from a copy too.** E1g saved each edit into memory and rebased onto it, which cleared the model's pending changes: no plan could report a `Conflict`, and an edit a certification forbids failed its in-memory save and was undone. The worker now writes a copy of every change since the last save after each edit, and PDFium opens the saved file with it, so the model's plan sees the session's changes. The copy overrides the signatures it breaks, for drawing only.
- **No `rebase` step and no `readableSpool`.** Adopting a copy rebases onto bytes already in memory, so the document is never `SavedNotRebased`. Saving doesn't stream into the File System Access writable yet: the copy is one buffer, as `saveAs` already wrote it, and an OPFS spool waits for files too large to hold.
- **`SaveResult.file`** ([ADR 0010](../../decisions/0010-platform-adapter.md)), not `download`: the platform answers the file as saved (read again after a write in place, since the one read before is stale; the new file after a save dialog), and the engine reads it from then on. After a download it reads the bytes downloaded, so the next save appends to them.
- **A restart after a save** reopens the saved file and replays only the steps since; an undo back past the save makes the steps unreplayable, as a partial undo already did.
- **Signatures are named by kind** ("The certification", "The signature (1 of 2)"): the plan's field is an internal ID, not the name a person gave it.
- **Save as optimised on a signed file** stays in the menu and says why it isn't available when chosen, rather than being disabled with the reason: the command system has no save plan to read.
- **Left for later:**
  - O01's "Saved · 16:20" divider in the History tab, and "Saved" showing for 2 s;
  - O02's Save a copy… button in the conflict strip, and each conflicting change with Show;
  - the user-facing help page, since the site has no reader help yet: the strip and the dialogs explain saving where it happens.
- **pdf.js:** files saved this way, rotated, open in pdf.js 5 with page 1 at 90°: a plain file and an approval-signed one (checked by hand with `tools/differential`'s pdf.js).
- **Startup script:** 273,839 bytes gzipped (limit 275,000). The save UI loads with the rest of the edit session (later.ts); the new message strings are in the startup catalogue.
