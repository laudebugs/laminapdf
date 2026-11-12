# Q4j — Starlight upgrades one minor at a time, with the overrides listed

| Field | Value |
| --- | --- |
| ID | `Q4j` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md) (the docs site's tooling) |
| Branch | `docs/q4j-starlight-policy` |
| PR title | `docs: a policy for upgrading Starlight and Astro` |
| Size | S |
| PR | — |

## Goal

The research recommends staying on Starlight, and names its main cost: each 0.x minor can break overrides, and Astro shipped two majors in 2026. This todo writes down how the docs site upgrades them, so upgrades stay small and the overrides stay few.

## Context

- [Docs research: recommendation 8](../../research/docs-site/developer-docs.md#recommendations-in-order) and the [frameworks note](../../research/docs-site/developer-docs/docs_frameworks.md).
- `apps/docs/package.json` (exact pins), `astro.config.mjs` (the `Header` and `MobileMenuFooter` overrides), `.github/dependabot.yml`.

## Scope

**In scope**

- A section in `apps/docs/README.md`: exact pins, one Starlight or Astro minor per PR, the changelog read for each, the overrides listed with why each exists.
- Dependabot grouped so Starlight, Astro and their plugins arrive together, if they don't already.

**Out of scope**

- The upgrades themselves.

## Tasks

- [ ] List the overrides and the custom CSS that depends on Starlight's markup.
- [ ] Write the policy; adjust Dependabot.

## Acceptance criteria

- [ ] The README lists every override and the upgrade steps.

## Post-merge checks

None.

## Tests

None (docs and config).

## Docs to update

`apps/docs/README.md`.

## Risks and open questions

- None.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
