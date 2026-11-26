# E1j — Desktop: the Properties dialog and editing metadata

| Field | Value |
| --- | --- |
| ID | `E1j` |
| Status | `done` |
| Plan | [E1 editing foundation §2](../../plans/editor/e1-editing-foundation.md#2-the-operations) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1f](./E1f-metadata-ui-capability-page.md), [E1i](./E1i-desktop-save-in-place.md) |
| ADRs | Follows ADR 0005, 0007, 0008 |
| Branch | `feat/e1j-desktop-properties` |
| PR title | `feat(desktop): the Properties dialog, with metadata edits that save` |
| Size | M (200–400) |
| PR | [#660](https://github.com/laudebugs/laminapdf/pull/660) |

## Goal

The desktop's File ▸ Document properties… opens the Properties dialog the web reader has, editable as E1f made it, and its edits save in place through E1i. The menu item exists, but the desktop has no dialog behind it yet, and its engine doesn't read a document's metadata. This todo flips the desktop column of the capability matrix's metadata rows (J.01–J.03, J.10), which E1i was to flip before the work showed the dialog was missing.

## Context

- [E1f](../done/E1f-metadata-ui-capability-page.md): the editable dialog (`PropertiesDialog` with `editing`, `@laminapdf/ui/dialogs`) and the web's round-trip e2e tests of these rows
- [E1i](../done/E1i-desktop-save-in-place.md): saving in place on the desktop, which the edits go through
- [E1h](../done/E1h-desktop-edit-session.md): the engine's edit session, which applies the metadata ops
- [ADR 0008 §2](../../decisions/0008-desktop-shell.md#2-process-model-a-main-process-that-owns-files-and-a-sandboxed-engine-per-window): only the engine parses, so the dialog's facts come from it
- Load the `/tauri-v2` skill before any work

## Scope

**In scope**

- An engine message that reads what the dialog shows: the Info and XMP metadata (`read_metadata`), the XMP packet's text, and the facts the read-only dialog lists (version, page sizes, fonts).
- The desktop's Properties dialog, opened by File ▸ Document properties…, with the editing form, its edits applied through the engine's edit session.
- `apps/desktop/e2e/specs/properties.e2e.ts`: edit each metadata field, save, reopen in the app and read the file with pdf.js, for J.01–J.03 and J.10.

**Out of scope**

- The XMP tab's general editor on the desktop: [E3k](../ongoing/E3k-desktop-navigation-and-export.md).

## Tasks

- [x] The engine's metadata read and its protocol message.
- [x] The dialog in the desktop window, over the engine's answers.
- [x] `properties.e2e.ts`, tagged so `cargo xtask capabilities` flips the desktop column.

## Acceptance criteria

- [x] On the desktop, editing each metadata field in the Properties dialog, saving and reopening shows the values in Info and XMP; these e2e tests flip the desktop column of J.01–J.03 and J.10.

## Post-merge checks

None.

## Tests

WebdriverIO e2e on three OSes; engine tests of the metadata read.

## Docs to update

`apps/desktop` README; the E1 plan's to-dos; the full-editor plan's metadata to-do.

## Risks and open questions

- The dialog's facts come from the engine, not a document worker: the dialog must take them as a shape, not from the viewer's client.

## Notes

- **One call when the dialog opens.** In protocol 9, the engine answers `GetProperties` with the facts, the metadata and the XMP packet as one JSON string in `@laminapdf/ui`'s shapes (`crates/laminapdf-desktop-engine/src/properties.rs`): the pipe's format can't carry a self-describing value, and the main process passes the string on unread.
- **The XMP tab works too.** The engine's answer carries the packet, and Apply's ops go through the same edit session, so the tab isn't left waiting. Its desktop capability row (J.04) stays [E3k](../ongoing/E3k-desktop-navigation-and-export.md)'s, with its e2e test.
- **One home for Apply's ops.** `propertiesOps` (`@laminapdf/ui/dialogs`) turns the dialog's changes into ops for both apps, so the web reader no longer keeps its own copy.
- **The desktop's scenarios can carry capability tags.** `TS_SUITES` (`tools/xtask/src/capabilities/tags.rs`) lists `apps/desktop/e2e/specs` for the `desktop` surface. The full lane's `desktop-e2e` jobs run those scenarios, and its `gate` waits on them.
- **Checkboxes in the scenario are ticked from the page.** WebKit's driver can't click a visually hidden input or its label, as `toggleSetting` already found.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
