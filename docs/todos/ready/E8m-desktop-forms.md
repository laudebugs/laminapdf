# E8m — Desktop: form filling and data

| Field | Value |
| --- | --- |
| ID | `E8m` |
| Status | `ready` |
| Plan | [E8 plan §6](../../plans/editor/e8-form-filling.md#6-differences-between-the-shells) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E1h](../done/E1h-desktop-edit-session.md), [E8h](./E8h-xfa-detection-and-notice.md), [E8i](./E8i-form-actions-and-submit.md), [E8j](./E8j-validation-and-calculation-ui.md), [E8k](../ongoing/E8k-form-data-ui.md), [E8l](./E8l-phone-form-filling.md), [E8n](../done/E8n-flat-form-tools.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) |
| Branch | `feat/e8m-desktop-forms` |
| PR title | `feat(desktop): form filling and data in the desktop app` |
| Size | M (200–400): the engine's calls, native dialogs, submit and the WebdriverIO suites |
| PR | — |

## Goal

The desktop part of round E8 ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo), and the round's close. Forms fill, format, calculate and flatten in the engine; data files come and go through native dialogs; a confirmed submit leaves from the main process, so a server without CORS still receives it. With this merged, every E8 row's desktop column is at its target.

## Context

- [E8 plan §5 and §6](../../plans/editor/e8-form-filling.md#6-differences-between-the-shells)
- [P01 Submit confirmation](../../design/paper/forms.md#submit-confirmation)'s desktop notes
- The engine's sandbox has no network (ADR 0008 §5), so the main process sends the request

## Scope

**In scope**

- The engine runs the round's ops and functions
- Native dialogs for import and export; File › Form data in the menu bar
- `submitForm` in the desktop platform: the main process sends the body `submit_request` built, `https:` only, to the host confirmed in the dialog, for that one request; `canSubmitWithoutCors` true
- The signed-document suite: `would_violate` and the banner for every E8 op on each `fixtures/signed/` file
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): every E8 row's desktop column
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.01`, `E.02`, `E.03`, `E.16` → `write` (desktop); `E.04`, `E.07`, `E.08`, `E.09`, `E.17` → `read` (desktop); `E.05`, `E.06`, `E.10`, `E.11`, `E.13`, `E.14` → `round-trip` (desktop)

## Tasks

- [ ] The engine's calls and protocol messages
- [ ] Native dialogs and the menu
- [ ] The main-process submit, with its Tauri permission scoped to the confirmed request
- [ ] WebdriverIO specs `apps/desktop/e2e/specs/forms.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: fill every field type, format a date, total a column, export and import JSON, flatten, save and check
- [ ] A submit to a local `https:` test server without CORS arrives with exactly the confirmed fields; the main process refuses any other host
- [ ] On `certified-p2.pdf` a fill saves incrementally and flattening is refused

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/forms.e2e.ts`
- The signed-document suite in `crates/laminapdf-save/tests/`

## Docs to update

- `docs/capabilities/README.md` via `cargo xtask capabilities`
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
