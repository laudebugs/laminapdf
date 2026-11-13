# E7a — ADR 0022: content-stream editing model for existing content

| Field | Value |
| --- | --- |
| ID | `E7a` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B9c](../done/B9c-text-extraction-bindings-and-differential.md), [E6a](../ongoing/E6a-adding-content-feature-plan.md) |
| Dependency notes | [E6a](E6a-adding-content-feature-plan.md), `B9` ([pdf-library §B9](../../plans/pdf-library.md#b9-text-extraction): content reading and text extraction) |
| ADRs | Writes ADR 0022; builds on ADR 0023 (B9) and 0005 §6 |
| Branch | `docs/e7a-adr-0022-content-editing` |
| PR title | `docs(adr): 0022 content-stream editing model` |
| Size | M (200–600) |
| PR | — |

## Goal

Decide how lamina edits existing content: the IR (operators with object identity: text runs, paths, images, form XObjects, marked content), how identity survives edits and saves, the encoder (preserve unmodified bytes vs re-serialise), how text runs are detected (MCIDs first, heuristics second), font strategy (reuse subset, re-subset, substitute) and the "best-effort, labelled" policy with read-only fallback. E7 and E12 build on it.

## Context

- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content)
- [full-editor §6](../../plans/full-editor.md#6-design-summary) content-stream layer step 3
- [03 §11.2 Concrete requirements](../../research/pdf/03-text-and-fonts.md#11-implications-for-laminas-design) (editing)
- [06 §7 Difficulty map](../../research/pdf/06-feature-inventory.md#7-difficulty-map): editing existing text is rank 7
- [02 §2 Content streams](../../research/pdf/02-imaging-model.md#2-content-streams)
- [library B9](../../plans/pdf-library.md#b9-text-extraction) and ADR 0023

## Scope

**In scope**

- ADR 0022 in `docs/decisions/0022-content-stream-editing.md` with context, options (display-list IR vs token-patch vs full re-serialise), decision, consequences, alternatives
- A short spike (in `spikes/content-edit/`, throwaway) measuring IR parse/encode size and time on the corpus and the byte-preservation rate
- Update `docs/decisions/README.md`, architecture §3 table, CLAUDE.md ADR list

**Out of scope** (and which todo covers it)

- Implementation (E7c onwards)

**Editor facts**

- **Crates and packages:** (decision only) `laminapdf-content`
- **`SaveConstraint`:** decides how edited streams are saved (new stream objects in the overlay; shared streams copy-on-write)
- **Web vs desktop:** n/a
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Spike and measurements
- [ ] Write the ADR
- [ ] Review with the owner
- [ ] Index updates

## Acceptance criteria

- [ ] ADR accepted (owner sign-off) with measured numbers
- [ ] Spike README states the recommendation and numbers

## Post-merge checks

None.

## Tests

Spike only; no production code.

## Docs to update

- `docs/decisions/README.md`
- `docs/plans/architecture.md` §3 table row for 0022
- `CLAUDE.md` accepted ADR list
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Choosing IR granularity too fine makes encoding expensive; too coarse blocks redaction (E12) — the ADR must cover E12's needs

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
