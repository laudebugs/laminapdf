# U1a — ADR 0024: app shell UX (layout, modes, command registry, selection, page appearance, mobile)

| Field | Value |
| --- | --- |
| ID | `U1a` |
| Status | `done` |
| Plan | [UI/UX §6 U1. Shell foundations](../../plans/ui-ux.md#u1-shell-foundations) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C2a](./C2a-render-adrs.md) |
| Blocks | [C4a](./C4a-ui-chrome-and-platform.md), [C4e](../ready/C4e-reader-chrome.md), [U1b](../ready/U1b-command-registry-and-palette.md), [U1c](../ready/U1c-inverted-l-shell-and-modes.md), [U6a](./U6a-paper-tokens.md) |
| ADRs | Writes 0024; follows 0009 (React chrome + framework-free viewer) and 0010 (`Platform` capability flags) |
| Branch | `docs/u1a-adr-0024-app-shell-ux` |
| PR title | `docs(adr): 0024 app shell ux: layout, modes, command registry and page appearance` |
| Size | S (one ADR, < 300 lines of prose) |
| PR | [#229](https://github.com/laudebugs/laminapdf/pull/229) |

## Goal

Record the interaction model every shell and editor round builds into, so `@laminapdf/ui`, the desktop menus and the editor modes don't each invent their own: the docked inverted-L layout, modes with one secondary toolbar, the single command registry, selection surfaces (desktop toolbar, touch bottom bar, never over the OS menu), page appearance modes separate from UI themes, the phone layout, and the design-token layers. These are cheap to decide now and expensive to change after C4a–C7a ship.

## Context

- **Design (Paper):** [F05](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §4–§5](../../plans/ui-ux.md#4-principles) — the principles and design this ADR makes binding.
- [Research report, UX section](../../research/product/pdf-app-product-and-ux.md#docked-chrome-modes-and-a-palette-beat-floating-toolbars) — evidence (Figma UI3, Acrobat quick tools, Linear, Office ribbon) and recommendations R1–R8.
- [UI/UX patterns notes](../../research/product/notes/ui-ux-patterns.md) — sources; several (w3.org APG, web.dev, NN/g) were read only through search snippets and must be re-checked here.
- ADR 0009 and ADR 0010 (from [C2a](../done/C2a-render-adrs.md)) — this ADR extends them and must not contradict them.

## Scope

**In scope**

- `docs/decisions/0024-app-shell-ux.md` (Proposed → Accepted in this PR once reviewed): context, decision, consequences, alternatives (ribbon, floating panels, per-surface menus, CSS invert for dark pages).
- Decisions: layout regions and when each shows; the mode list and that a mode owns exactly one secondary toolbar; the `Command` shape (`id`, `title`, `keywords`, `group`, `shortcut`, `enabled(ctx)`, `run(ctx, args)`, typed `args`); the rule that every command has a visible home besides the palette; selection surfaces per pointer type; page modes (Original, Sepia, Dark, High contrast) and that annotation colours never change; phone layout (bottom bar, sheets with detents, viewer-owned gestures); token layers (primitive → semantic → component; a separate page-backdrop token).
- Accessibility rules: WCAG 2.2 AA targets (24 px, 44 px on touch), SC 2.5.7 single-pointer alternatives, SC 2.4.11 focus not obscured, APG combobox for the palette and toolbar pattern for the selection toolbar, each re-checked against the primary W3C pages.
- Rows in `docs/decisions/README.md` and [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs).

**Out of scope** (and which todo covers it)

- Implementation → [U1b](../ready/U1b-command-registry-and-palette.md), [U1c](../ready/U1c-inverted-l-shell-and-modes.md), [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md), [U2c](../ready/U2c-page-appearance-modes.md), [U3a](../ready/U3a-responsive-mobile-and-touch.md).
- Native mobile shell choice → ADR 0025 after [U3b](../ready/U3b-mobile-shell-spike.md).
- Visual identity: decided (owner, 2026-10-29) as the **Paper** direction; the ADR's token layers take their values from the [Paper design language](../../design/paper/design-language.md), and [U6a](../done/U6a-paper-tokens.md) builds them ([Paper design plan](../../plans/paper-design.md)). The brand mark stays open.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Re-read ADRs 0009 and 0010 as merged; list where 0024 depends on them.
- [x] Fetch and cite the primary sources the research could not reach (WCAG 2.2 SC 2.4.11, 2.5.7, 2.5.8; APG combobox, toolbar and grid patterns; Apple HIG sheets; Material 3 bottom sheets); correct the plan if they disagree.
- [x] Write the ADR with a layout diagram, the mode table, the `Command` type in TypeScript, the selection-surface table and the page-mode table.
- [x] List the reader v0.1 command set (about 30 commands: open, page navigation, zoom, rotate view, find, print, download, properties, toggle rail, focus mode, page modes, help …) as an appendix so U1b starts from it.
- [x] Add the ADR to the decisions index and architecture §3; link it from the UI/UX plan.

## Acceptance criteria

- [x] The ADR covers every decision listed in scope, each with at least one rejected alternative.
- [x] Every WCAG/APG claim cites the W3C page directly (not a secondary source).
- [x] `cargo xtask check docs` passes.

## Post-merge checks

None.

## Tests

None (docs only); the link checker covers the new links.

## Docs to update

`docs/decisions/0024-app-shell-ux.md` (new), `docs/decisions/README.md`, [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs), tick "Write ADR 0024" in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Deciding modes before the editor exists: keep the list open-ended (a mode is registered, not hard-coded) and only Read ships in v0.1.
- If ADR 0009 picked a primitives library whose toolbar or combobox semantics differ from APG, note the gap and the workaround here.

## Outcome

- [ADR 0024](../../decisions/0024-app-shell-ux.md) is accepted with seven decisions plus the accessibility rules that hold across them, eleven rejected alternatives and the reader v0.1 command set (39 commands) as its appendix.
- It depends on ADR 0009 for React Aria Components and `ReaderController`, where modes register. It depends on ADR 0010 for the capability rule commands follow: hidden where the platform can never have a capability, disabled with a reason where the user could unlock it.
- Every WCAG 2.2, APG, Apple HIG and Material 3 claim cites the primary page, checked on 2026-11-04; they agree with the UI/UX plan. Two findings went into the ADR: SC 2.5.7 covers the viewer's own panning, and a keyboard equivalent alone doesn't meet it.
- The design language's command-id appendix gains `mode.comment`, `view.set-zoom` and `view.rotate-display`; ADR 0010 gains the `ownsKeyboard` flag and `Platform.howToEnable`, which the command registry needs. Token CSS names keep the docs site's `--paper-` prefix.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
