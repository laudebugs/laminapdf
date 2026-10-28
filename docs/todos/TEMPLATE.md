# <ID> — <Title>

<!--
Copy this file to docs/todos/ready/<ID>-<kebab-slug>.md (e.g. B2b-decryption.md).
One todo = one pull request. If the work is bigger than one reviewable PR, split it
into several todos (B2a, B2b, …) and chain them with "Depends on".
Keep the section order; delete a section only if it truly doesn't apply (write "None").
-->

| Field | Value |
| --- | --- |
| ID | `<ID>` |
| Status | `ready` \| `ongoing` \| `done` (must match the folder) |
| Plan | [<plan name> §<section>](../../plans/<plan>.md#<anchor>) |
| Round / milestone | <e.g. Round B (library) — M1> |
| Depends on | <todo IDs that must be **done** (merged) first, linked; or "None"> |
| Blocks | <todo IDs that wait for this one> |
| ADRs | <ADRs this todo writes, implements or must follow> |
| Branch | `<type>/<id>-<slug>` (e.g. `feat/b2b-decryption`) |
| PR title | `<conventional commit title>` |
| Size | S (< 200 lines) \| M (200–600) \| L (600–1,500) \| XL (split it) |
| PR | <link once opened; "—" while ready> |

## Goal

One paragraph: what this PR delivers and why it matters now.

## Context

Links to the research chapters, ADRs, spike results, earlier PRs and spec clauses (ISO 32000) a contributor must read first, with one line on why each matters.

## Scope

**In scope**

- …

**Out of scope** (and which todo covers it)

- …

## Tasks

A checklist detailed enough that someone new to the codebase could do the work in order: crates/packages/files to create or change, types and functions to add, data to generate, config to touch.

- [ ] …

## Acceptance criteria

Observable, checkable conditions (tests that must exist and pass, tools that must accept the output, numbers that must be met). CI green is implied.

- [ ] …

## Tests

Which tests to add (unit, integration, golden, corpus, differential, fuzz, browser/e2e), where they live, and what fixtures they need.

## Docs to update

Plans (tick the to-dos), ADR status, CLAUDE.md, READMEs, guidelines — whatever this PR changes.

## Risks and open questions

What could go wrong, what's unknown, what to decide during the work (and who decides).

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
