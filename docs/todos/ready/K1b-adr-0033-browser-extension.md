# K1b — ADR 0033: the browser extension

| Field | Value |
| --- | --- |
| ID | `K1b` |
| Status | `ready` |
| Plan | [Browser extension §5 K1](../../plans/browser-extension.md#k1-prove-it-decide-it-draw-it) |
| Round / milestone | Round K (browser extension) — M2b Browser extension |
| Depends on | [K1a](./K1a-browser-extension-spike.md) |
| ADRs | Writes 0033. Follows [0009](../../decisions/0009-web-client-architecture.md), [0010](../../decisions/0010-platform-adapter.md), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md), [0017](../../decisions/0017-versioning-and-release.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `docs/k1b-adr-0033-browser-extension` |
| PR title | `docs(adr): 0033 the browser extension` |
| Size | S (< 200 changed lines: one ADR and its index entries) |
| PR | — |

## Goal

Record how Lamina PDF's extension is built and shipped, so K2a to K4a build against one decision. The ADR settles the extension's shape: the reader bundled inside it, Manifest V3, WXT, the `browser.*` namespace, the permission model, the privacy rules, how releases are versioned and signed, and the order of the stores. It uses K1a's measurements for every choice the research left open.

## Context

- [Browser extension research](../../research/browser-extension/browser-extension.md) and [plan §2 and §3](../../plans/browser-extension.md#2-what-the-research-found): the findings and the round's rules.
- K1a's README in `spikes/browser-extension/`: the answers to plan §9.
- [Permissions note](../../research/browser-extension/notes/permissions-privacy-enterprise.md): install warnings per permission, `data_collection_permissions`, signed CRX uploads.
- [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md): the ten privacy guarantees the extension must keep or restate.
- [ADR 0010](../../decisions/0010-platform-adapter.md): the extension is a new `Platform` with its own capability flags.

## Scope

**In scope**

- ADR 0033 covering:
  - the package: the reader bundled inside the extension;
  - Manifest V3, with a background service worker in Chromium and an event page in Firefox;
  - WXT, and `minimum_chrome_version`;
  - the permission set at install and the optional permissions;
  - the privacy rules: no remote code, no telemetry, the CSP;
  - takeover per engine, and the fallback;
  - versioning (the extension's own version or the lockstep one) and signing-key custody;
  - store order.
- The decisions index and CLAUDE.md's list of accepted ADRs.

**Out of scope** (and which todo covers it)

- Screens: [K1c](./K1c-extension-screens-in-paper.md).
- Any code: [K2a](./K2a-extension-app-and-manual-open.md) onwards.

## Tasks

- [ ] Draft ADR 0033 from the decisions template, with options considered and K1a's numbers.
- [ ] Decide whether the extension follows the lockstep version (ADR 0017) or its own, and say why.
- [ ] Decide whether Firefox ships PDFium or hayro alone, from K1a's ARM64 result.
- [ ] Add the ADR to `docs/decisions/README.md` and CLAUDE.md's accepted list.

## Acceptance criteria

- [ ] ADR 0033 is accepted and lists every permission the extension may ask for, each with its reason and whether it is optional.
- [ ] The ADR states which of ADR 0011's guarantees apply unchanged, and restates those that change.

## Post-merge checks

None.

## Tests

None; `cargo xtask check docs` checks the links.

## Docs to update

- `docs/decisions/README.md`, CLAUDE.md, and the plan (tick K1b).

## Risks and open questions

- If K1a finds that takeover in Firefox fails, the ADR may make Firefox button-first like Safari. The owner decides before acceptance.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
