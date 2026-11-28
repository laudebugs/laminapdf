# E11f — Protect dialog and encryption inspector

| Field | Value |
| --- | --- |
| ID | `E11f` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E11b](./E11b-encrypt-on-write.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) |
| Branch | `feat/e11f-protect-dialog` |
| PR title | `feat(ui): the Protect dialog and the encryption inspector` |
| Size | L (400–800) |
| PR | — |

## Goal

The screens for E11b's encryption: the Protect dialog's Password tab, the status-bar security item, and the inspector panel that shows how a file is protected. Opens the web and desktop columns of G.01 to G.04 and G.11.

## Context

- **Design (Paper):** [G06 Protect](../../design/paper/sign-protect-redact.md#g06-protect). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E11 feature plan](../../plans/editor/e11-security-and-sanitisation.md) §2.1 (passwords, permissions mapping), §5 (the signed-file rows) and §6 (the shells)
- [E11b](./E11b-encrypt-on-write.md): the `SetEncryption` op, `encryption_info()` and the permission reasons this PR draws

## Scope

**In scope**

- The Protect dialog (Password tab): open password and confirmation with show/hide toggles, the permissions password, the Printing select and the allowed-actions checkboxes (the plan's §2.1 table), "Also encrypt the title, author and other document information", the honest note that the limits are advisory
- The signed-file notice and the **Save a protected copy…** and **Remove signature and protect** actions (plan §5)
- The PDF/A notice and **Remove the PDF/A claim and protect** (plan §3)
- The status-bar security item and the encryption inspector panel (handler, revision, cipher, key length, permissions, which password opened the file; no key material)
- The `security.protect` command, its place on the Edit toolbar and in the palette, and its phone large-detent sheet
- Help page: protecting a PDF

**Out of scope** (and which todo covers it)

- The remove-protection dialog: [E11c](./E11c-remove-security.md)
- The Certificates tab: [E11e](./E11e-public-key-security.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.01`, `G.02`, `G.03`, `G.04`, `G.11` (the screens)
- **Crates and packages:** `@laminapdf/ui`, `apps/web`, `apps/desktop`
- **`Op` variants:** none new
- **`SaveConstraint`:** as E11b's; this PR draws the strips the plan's §5 names
- **Web vs desktop:** identical; passwords stay in the worker and never reach a log, a toast or an error message
- **Server features:** none
- **Capability-matrix rows to flip:** `G.01`, `G.02`, `G.03`, `G.04` → `round-trip` (web, desktop); `G.11` → `read` (web, desktop)

## Tasks

- [ ] The dialog and its validation states (passwords don't match, empty)
- [ ] The signed-file and PDF/A notices and their actions
- [ ] The status-bar item and the inspector panel
- [ ] The command, toolbar button, palette entry and phone sheet
- [ ] Playwright and WebdriverIO scenarios; axe on each screen
- [ ] Help page

## Acceptance criteria

- [ ] A person can protect a file with each password and each permission, save, reopen it with the right password and see the inspector's facts; the wrong password fails
- [ ] The dialog never shows or logs a password outside its own fields; a test greps the page, the console and the toasts for the fixture passwords
- [ ] The signed-file notice names each signature; **Save a protected copy…** leaves the original untouched
- [ ] Every control is reachable and named by keyboard and screen reader (axe clean); the note that bit 10 stays on is in the dialog

## Post-merge checks

None.

## Tests

- Vitest for the dialog state and the permission mapping
- Playwright and WebdriverIO: protect, save, reopen; the signed-file path

## Docs to update

- User help: protecting a PDF
- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) and its §10 to-dos
- `docs/capabilities/manifest.toml` statuses (regenerated via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
