# Q4n — "Added in" markers on API items, and stable docs per release from 1.0

| Field | Value |
| --- | --- |
| ID | `Q4n` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0017](../../decisions/0017-versioning-and-release.md) (versions and the release and next docs) |
| Branch | `docs/q4n-since-markers` |
| PR title | `docs: mark when each API item was added or changed` |
| Size | M |
| PR | [#473](https://github.com/laudebugs/laminapdf/pull/473) |

## Goal

Developers praise Django for saying in which version something changed, and PostgreSQL for keeping old versions' docs online. Lamina PDF is still 0.x with daily releases, so this todo starts small: "Added in" and "Changed in" markers on API items, from the release history, and a written decision on keeping each 1.x release's docs at a stable URL once 1.0 ships.

## Context

- [Docs research: recommendation 11](../../research/docs-site/developer-docs.md#recommendations-in-order); its strength is "weak, opinion", since the versioning counts were unusable.
- `fixtures/codes.json`'s `since` field (the codes already have it), the changelog, `docs/renames.json`, starlight-typedoc's output.

## Scope

**In scope**

- Markers in the TypeScript reference and the Rust docs from a `@since`-style tag.
- A note in the releases plan on per-version docs from 1.0.

**Out of scope**

- Building a version switcher now.

## Tasks

- [x] Choose the tag (TSDoc `@since`, rustdoc text) and fill it from the changelog for the public API. The owner chose (#474) the first release after the alphas, `0.2.0`, for every item that exists today, in each doc comment, in both languages.
- [x] Show it in the TypeDoc pages; add a check that new public items carry one.
- [x] Write the 1.0 decision ([releases plan §4](../../plans/releases.md#4-docs-that-match-a-version)).

## Acceptance criteria

- [x] Every public TypeScript and Rust item has an "Added in" version.
- [x] A new public item without one fails a check.

## Post-merge checks

None.

## Tests

The new check; the docs build.

## Docs to update

The releases plan; the docs README.

## Risks and open questions

- Backfilling versions by hand is error-prone; derive them from git where possible.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
