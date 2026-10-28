# E1e — Save, Save as, Download copy and the SaveConstraint banner

| Field | Value |
| --- | --- |
| ID | `E1e` |
| Status | `ready` |
| Plan | [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3b](./D3b-recent-files-and-file-watching.md), [E1c](./E1c-metadata-ops.md), [E1d](./E1d-edit-session-undo-ui.md) |
| Blocks | [E1f](./E1f-metadata-ui-capability-page.md), [E2h](./E2h-page-organiser-ui.md), [E4f](./E4f-annotation-tools-ui.md), [E8e](./E8e-field-widgets-ui.md), [H3](./H3-cloud-document-storage.md) |
| Dependency notes | [E1c](E1c-metadata-ops.md) (`SaveOptions { metadata_date, instance_id }`, `ConformanceWarning`, `SaveReport::metadata_not_updated`), [E1d](E1d-edit-session-undo-ui.md), `D3` ([desktop-app §D3](../../plans/desktop-app.md#d3-files): files and watching) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1e-save-and-constraint-banner` |
| PR title | `feat(app): save through the platform adapter with the save-constraint banner` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Let users save edits safely on both shells: Save (incremental by default), Save as, Save as optimised (full rewrite) and Download copy through the `Platform` adapter, with a banner that shows the document's `SaveConstraint` (Free, AppendOnly, RequiresRewrite, Conflict), names every signature a save would invalidate, and asks before any override.

## Context

- [ADR 0007](../../decisions/0007-save-planner.md): planning vs writing, `SaveConstraint.kind()`, override, sinks and rebase
- [ADR 0005](../../decisions/0005-document-model.md) §3: rebase after save and `SavedNotRebased`
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) "Web vs desktop"; [full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 2
- [desktop D3](../../plans/desktop-app.md#d3-files): file watching and reload on external change

## Scope

**In scope**

- Save commands: `Save`, `Save as…`, `Save as optimised` (full rewrite), `Download a copy`
- Web: File System Access API (`showSaveFilePicker`, `createWritable`) where available; streamed `WriteSink` into it; otherwise download via Blob URL; OPFS spool for full rewrites (readable sink for rebase)
- Desktop: in-place save with `IncrementalOutput::AppendedOnly` append for incremental saves (atomic temp-file + rename for full rewrites); suppress the file watcher for our own writes; external change while dirty → conflict dialog (reload / save as / overwrite)
- `SaveConstraint` banner component: kind, signatures (field, signer, kind), rewrite reasons, violations; explanation texts; confirm dialog naming each signature before `invalidate_signatures: true`
- Show `ConformanceWarning`s from E1c before saving, and say when save-time metadata was left unchanged to keep a certification valid (`metadata_not_updated`)
- Pass save-time inputs (`metadata_date`, `instance_id`) from the shell (clock at the edge)

**Out of scope** (and which todo covers it)

- Capability-matrix page and metadata UI → [E1f](E1f-metadata-ui-capability-page.md)
- Optimisation options beyond a plain full rewrite → [E16b](E16b-structural-optimiser.md)

**Editor facts**

- **Crates and packages:** `lamina-save` (sinks), `lamina-wasm` (streaming sink adapter), `@lamina/ui` (banner, dialogs), `apps/web` and `apps/desktop` platform adapters
- **`SaveConstraint`:** implements the UI for all four kinds: `Free` → incremental default; `AppendOnly` → incremental only, full rewrite blocked; `RequiresRewrite` → full rewrite, old revisions dropped (said in the dialog); `Conflict` → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** Desktop saves in place (`canWriteInPlace: true`) and watches for external changes (D3); web saves in place only where the File System Access API exists, otherwise downloads a copy. Web without FSA API: "Save" becomes "Download" and the document rebases onto what was downloaded
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (infrastructure); unblocks `write`/`round-trip` claims for every later op in web and desktop columns

## Tasks

- [ ] Extend the `Platform` adapter (ADR 0010) with `saveInPlace`, `saveAs`, `download`, `canWriteInPlace` already defined; add `readableSpool()` for rebase
- [ ] Web: streaming writable sink in the worker; fallback download path; OPFS spool; tests for Chromium (FSA) and Firefox/WebKit (download)
- [ ] Desktop (load `/tauri-v2` first): Rust commands for append-in-place and atomic rewrite; watcher suppression token; conflict detection by size+mtime+hash of the base
- [ ] Banner and dialogs in `@lamina/ui` with copy reviewed against ADR 0007 wording
- [ ] Rebase handling: after a full rewrite to a non-readable sink, call `rebase` with the spooled bytes; block ops while `SavedNotRebased`
- [ ] Error paths: disk full, permission denied, FSA permission revoked → document stays dirty, overlay untouched

## Acceptance criteria

- [ ] e2e: edit → Save → reopen in the app and in pdf.js shows the change; file grew only by the appended revision (incremental) on desktop
- [ ] On a signed fixture the banner shows AppendOnly with the signer; "Save as optimised" is blocked; a DocMDP-violating op shows the named signature and requires confirmation
- [ ] On a desktop file edited externally while dirty, the conflict dialog appears; on a clean document the file reloads keeping page position
- [ ] A failed write leaves the document dirty and the original file intact (desktop test with a read-only file)
- [ ] Web: Chromium saves in place via FSA; Firefox and WebKit download a copy

## Post-merge checks

None.

## Tests

- Playwright: `apps/web/e2e/save.spec.ts` (FSA mocked in Chromium, download path elsewhere)
- WebdriverIO: `apps/desktop/e2e/save.e2e.ts` incl. external-change conflict
- Rust tests for append-in-place and atomic rewrite in `src-tauri`
- Signed fixtures for banner states

## Docs to update

- User-facing help page on saving and signatures
- ADR 0010 if the adapter gains methods
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- FSA API permission prompts differ across Chromium versions; keep the download fallback always available
- Append-in-place on network drives/cloud-synced folders may be non-atomic; recommend Save as for those (detect where possible)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
