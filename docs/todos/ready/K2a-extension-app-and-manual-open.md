# K2a — `apps/extension`: the reader in a WXT extension, opened by hand

| Field | Value |
| --- | --- |
| ID | `K2a` |
| Status | `ready` |
| Plan | [Browser extension §5 K2](../../plans/browser-extension.md#k2-chromium-chrome-edge-brave-vivaldi-opera) |
| Round / milestone | Round K (browser extension) — M2b Browser extension |
| Depends on | [K1b](./K1b-adr-0033-browser-extension.md), [K1c](./K1c-extension-screens-in-paper.md) |
| ADRs | Implements 0033. Follows [0009](../../decisions/0009-web-client-architecture.md), [0010](../../decisions/0010-platform-adapter.md) (an extension `Platform`), [0015](../../decisions/0015-monorepo-layout-and-tooling.md), [0018](../../decisions/0018-test-strategy.md) |
| Branch | `feat/k2a-extension-app` |
| PR title | `feat(extension): package the reader as a browser extension with manual open` |
| Size | L (400–800 changed lines: a new app, its platform adapter and its browser tests) |
| PR | — |

## Goal

Ship the extension's first working build for Chromium, without taking anything over yet. A new `apps/extension` packages the reader with WXT. The user opens a PDF from the toolbar popup, from the context menu on a link ("Open in Lamina PDF"), or from a file picker, and the reader opens it in a tab. Because the viewer is an extension page, it fetches remote PDFs without needing CORS. This build asks for no permission that shows an install warning.

## Context

- ADR 0033 and K1a's README: the package layout, the CSP and what had to change in the reader.
- K1c's area document: the popup screen.
- `apps/web/README.md` and `src/session.ts`: the reader's engine; the extension reuses it rather than copying it.
- [ADR 0010](../../decisions/0010-platform-adapter.md): the extension's `Platform` adapter and capability flags.

## Scope

**In scope**

- `apps/extension/` (private, outside Changesets) with WXT: the manifest, the viewer page importing the reader, the popup, the context-menu item, and the background script.
- What the reader and the extension share moves into a package or an exported entry of `apps/web`, whichever ADR 0033 chose.
- An extension `Platform` adapter.
- `activeTab`-based fetch for the context menu, so no host permission is needed at install.
- Playwright tests in Chromium with the unpacked extension: open by popup, by context menu, and by file; no CSP violation; no request to any site but the PDF's.
- A `cargo xtask` entry, or a step in `cargo xtask web build`, that builds the extension.

**Out of scope** (and which todo covers it)

- Automatic takeover and the onboarding page: [K2b](./K2b-automatic-pdf-takeover-chromium.md).
- Store packaging and release: [K2c](./K2c-chromium-release.md).
- Firefox and Safari builds: [K3a](./K3a-firefox-extension.md), [K4a](./K4a-safari-extension.md).

## Tasks

- [ ] Scaffold `apps/extension` with WXT; add it to the pnpm workspace and to `.changeset/config.json`'s `ignore`.
- [ ] Share the reader's engine and chrome with the extension without copying code.
- [ ] Build the popup from K1c's design, the context-menu item and the file picker.
- [ ] Write the extension `Platform` adapter.
- [ ] Add the Playwright suite and wire it into the lane ADR 0018 picks.
- [ ] Try it in Chrome as a user would, following the browser-test working agreement.

## Acceptance criteria

- [ ] In Chrome, the user opens a PDF from a host that sends no CORS headers through the context menu, and it renders.
- [ ] The installed extension shows no install warning, which the manifest's permission list confirms.
- [ ] The Playwright suite passes and records zero requests outside the PDF's origin.

## Post-merge checks

None.

## Tests

Playwright with the unpacked extension in Chromium, under `apps/extension/e2e/`; unit tests for the adapter in Node.

## Docs to update

- `apps/extension/README.md`, CLAUDE.md's layout, and the plan (tick K2a).

## Risks and open questions

- Sharing the reader may need `apps/web` split into a package; if so, that split may be a todo of its own before this one.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
