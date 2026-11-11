# E1e — Save, Save as, Download copy and the SaveConstraint banner

| Field | Value |
| --- | --- |
| ID | `E1e` |
| Status | `ready` |
| Plan | [E1 editing foundation §7](../../plans/editor/e1-editing-foundation.md#7-saving-and-the-save-constraint) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1c](../ongoing/E1c-metadata-ops.md), [E1d](./E1d-edit-session-undo-ui.md) |
| Dependency notes | [E1c](../ongoing/E1c-metadata-ops.md) (`SaveOptions { metadata_date, instance_id }`, `ConformanceWarning`, `SaveReport::metadata_not_updated`), [E1d](E1d-edit-session-undo-ui.md). The desktop's save in place is [E1i](./E1i-desktop-save-in-place.md), after D3b; the constraint table is [E1 plan §7](../../plans/editor/e1-editing-foundation.md#7-saving-and-the-save-constraint) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1e-save-and-constraint-banner` |
| PR title | `feat(app): save through the platform adapter with the save-constraint banner` |
| Size | L (600–1,500) |
| PR | — |

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
- Desktop: in-place save and outside changes → [E1i](./E1i-desktop-save-in-place.md)
- `SaveConstraint` banner component: kind, signatures (field, signer, kind), rewrite reasons, violations; explanation texts; confirm dialog naming each signature before `invalidate_signatures: true`
- Show `ConformanceWarning`s from E1c before saving, and say when save-time metadata was left unchanged to keep a certification valid (`metadata_not_updated`)
- Pass save-time inputs (`metadata_date`, `instance_id`) from the shell (clock at the edge)

**Out of scope** (and which todo covers it)

- Capability-matrix page and metadata UI → [E1f](E1f-metadata-ui-capability-page.md)
- Optimisation options beyond a plain full rewrite → [E16b](E16b-structural-optimiser.md)

**Editor facts**

- **Crates and packages:** `laminapdf-save` (sinks), `laminapdf-wasm` (streaming sink adapter), `@laminapdf/ui` (banner, dialogs), `apps/web`'s platform adapter
- **`SaveConstraint`:** implements the UI for all four kinds: `Free` → incremental default; `AppendOnly` → incremental only, full rewrite blocked; `RequiresRewrite` → full rewrite, old revisions dropped (said in the dialog); `Conflict` → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** web here: in place only where the File System Access API exists, otherwise Save becomes Download and the document rebases onto what was downloaded. The desktop's save in place is E1i
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (infrastructure); unblocks `write`/`round-trip` claims for every later op in web and desktop columns

## Tasks

- [ ] Extend the `Platform` adapter (ADR 0010) with `saveInPlace`, `saveAs`, `download`, `canWriteInPlace` already defined; add `readableSpool()` for rebase
- [ ] Web: streaming writable sink in the worker; fallback download path; OPFS spool; tests for Chromium (FSA) and Firefox/WebKit (download)
- [ ] Banner and dialogs in `@laminapdf/ui` with copy reviewed against ADR 0007 wording
- [ ] Rebase handling: after a full rewrite to a non-readable sink, call `rebase` with the spooled bytes; block ops while `SavedNotRebased`
- [ ] Error paths: disk full, permission denied, FSA permission revoked → document stays dirty, overlay untouched

## Acceptance criteria

- [ ] e2e: edit → Save → reopen in the app and in pdf.js shows the change
- [ ] On a signed fixture the banner shows AppendOnly with the signer; "Save as optimised" is blocked; a DocMDP-violating op shows the named signature and requires confirmation
- [ ] A failed write leaves the document dirty and the original file intact (a refused File System Access write in Chromium)
- [ ] Web: Chromium saves in place via FSA; Firefox and WebKit download a copy

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
