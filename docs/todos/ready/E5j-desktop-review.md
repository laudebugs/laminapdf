# E5j — Desktop: review, stamps and interchange

| Field | Value |
| --- | --- |
| ID | `E5j` |
| Status | `ready` |
| Plan | [E5 plan §6](../../plans/editor/e5-review-stamps-interchange.md#6-differences-between-the-shells) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4h](./E4h-desktop-annotations.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md), [E5g](./E5g-comments-panel.md), [E5h](./E5h-stamps-and-attachments-ui.md), [E5i](./E5i-interchange-flatten-summary-ui.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e5j-desktop-review` |
| PR title | `feat(desktop): review, stamps and interchange in the desktop app` |
| Size | M (200–400): the engine's calls, native dialogs and the WebdriverIO suites |
| PR | — |

## Goal

The desktop part of round E5 ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo), and the round's close. The round's ops and functions run in the engine, files come and go through native dialogs, custom stamps live in the app-data folder, and the summary opens in a new window. With this todo merged, every E5 row's desktop column is complete.

## Context

- [E5 plan §5 and §6](../../plans/editor/e5-review-stamps-interchange.md#6-differences-between-the-shells)
- [E4h](E4h-desktop-annotations.md): Comment mode on the desktop

## Scope

**In scope**

- The engine runs the round's ops, `comment_threads`, interchange, flattening and the summary
- Native dialogs for attachments, imports and exports; custom stamps in the app-data folder; the summary in a new window
- The signed-document suite: `would_violate` and the banner for every E5 op on each `fixtures/signed/` file
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): every E5 row's desktop column
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.07`, `D.08`, `D.10`, `D.11`, `D.13`, `D.14`, `D.15` → `round-trip` (desktop); `D.09`, `D.12` → `read` (desktop)

## Tasks

- [ ] The engine's calls
- [ ] Native dialogs and the stamp folder
- [ ] The signed-document suite
- [ ] WebdriverIO specs `apps/desktop/e2e/specs/review.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: reply, set a status, place a stamp, attach a file, export and import an XFDF, flatten a page and make a summary, each saved and checked
- [ ] On the certified-P3 fixture a reply saves incrementally and flattening is refused

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/review.e2e.ts`
- The signed-document suite in `crates/laminapdf-save/tests/`

## Docs to update

- `docs/capabilities/README.md` via `cargo xtask capabilities`
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
