# E8i — Push buttons, form actions and submit

| Field | Value |
| --- | --- |
| ID | `E8i` |
| Status | `ready` |
| Plan | [E8 plan §2.3 and §6](../../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8e](../ongoing/E8e-field-widgets-ui.md), [E8f](../done/E8f-declarative-behaviour.md), [E8g](../done/E8g-form-data-and-flatten.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md) (amends: `submitForm`, `canSubmitWithoutCors`), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the owner's 2026-11-17 amendment lands first), [0021](../../decisions/0021-pdf-javascript-policy.md) |
| Branch | `feat/e8i-form-actions-and-submit` |
| PR title | `feat(forms): push buttons, form actions and confirmed submit` |
| Size | L (400–800 of code; e2e specs beyond) |
| PR | — |

## Goal

Push buttons work without scripts. The library reads a field's actions as typed data. The reader runs only the safe ones (go to a page, open a link after asking, reset, hide), names the rest, and sends a SubmitForm only after a dialog that shows every field and value that leaves the device, never the document itself.

## Context

- **Design (Paper):** [P01 Submit confirmation](../../design/paper/forms.md#submit-confirmation). Build to match it; record any deviation in the area document.
- [E8 plan §2.3 and §6](../../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit), decision 7 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- ISO 32000-2 §12.6 (actions), §12.7.6.2 (submit-form flags and formats), §12.7.6.3 (reset-form), §12.7.6.4 (import-data)
- [full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 6 (privacy); ADR 0011 §3's guarantees 1, 3 and 5

## Scope

**In scope**

- ADR 0011 amendment first: its PR, a submit only after the person confirms that one send, seeing exactly what is sent and where, off by default (decided by the owner on 2026-11-17), merges before this PR
- `form_actions(field)`: `/A` and `/AA` through `/Next` chains, within `Limits`, as typed actions
- The reader's action runner per the plan's table; JavaScript, Launch, GoToR, GoToE and ImportData named, never run
- `submit_request(action)` → URL, format (HTML form encoding, FDF, XFDF), body, blocking empty required fields; `submit-pdf-refused`; `https:` only; `mailto:` opens the mail app without data
- `Platform.submitForm` and `canSubmitWithoutCors` in ADR 0010's tables (web value `false`); the web implementation by `fetch` with `credentials: "omit"` and no referrer
- P01's submit dialog and its three failures; the network indicator lists the request
- `fixtures/forms/actions.pdf`

**Out of scope** (and which todo covers it)

- The desktop's submit from the main process → [E8m](E8m-desktop-forms.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.04` push buttons; `E.10` conditional behaviour (from buttons and triggers); `E.16` submit to external endpoints
- **Crates and packages:** `laminapdf-model`, `laminapdf-interchange`, the façade and bindings, `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none new; actions lead to `SetFieldValue`, `SetFieldHidden` or E4's `UpdateAnnot`
- **`SaveConstraint`:** each action's ops as their own; submit changes nothing
- **Web vs desktop:** `canSubmitWithoutCors` false on the web: a server without CORS gets P01's "Not sent" message
- **Server features:** none: a submit goes from the device to the form's address, after the user confirms that send
- **Capability-matrix rows to flip:** `E.04` → `read` (core, pdf, web); `E.10` → `round-trip` (web); `E.16` → `write` (core, pdf, web)

## Tasks

- [ ] `form_actions` and `submit_request` with façade and SDK parity
- [ ] The action runner
- [ ] The `Platform` method and flag, ADR 0010's tables
- [ ] The submit dialog
- [ ] Playwright specs with network assertions

## Acceptance criteria

- [ ] The ADR 0011 amendment ([E8 plan §10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions), question 7) is merged before this PR
- [ ] Playwright: no request leaves before Send; the request carries exactly the listed fields; SubmitPDF and `http:` are refused; a JavaScript button says it didn't run
- [ ] Each submit format's body matches its golden for `fields.pdf`

## Post-merge checks

None.

## Tests

- Unit tests per action and submit flag; goldens of the bodies; Playwright in `apps/web/e2e/`

## Docs to update

- ADR 0010's tables; ADR 0011 §3 (the amendment, linked)
- User help: buttons and sending forms
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Many form servers send no CORS headers, so web submits often fail; the dialog's way out is Export form data… or the desktop app

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
