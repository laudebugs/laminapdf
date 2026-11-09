# C5c — Find's Results tab in the rail

| Field | Value |
| --- | --- |
| ID | `C5c` |
| Status | `done` |
| Plan | [web reader §6 C5. Text, selection and search](../../plans/web-app.md#c5-text-selection-and-search) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C5b](./C5b-find-in-document.md), [C6a](./C6a-thumbnails-outline-page-labels.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (the registry's command list stays as it is) |
| Branch | `feat/find-results-tab` |
| PR title | `feat(ui): list find's results in the rail` |
| Size | L (400–800) |
| PR | [#353](https://github.com/laudebugs/laminapdf/pull/353) |

## Goal

While find is open, the rail gains a **Results** tab that lists every match in page order: grouped under the page's label and its outline section, each a link with a snippet of the text around it, the current match marked. Clicking one goes to it. C5b shipped the find bar and left this tab for when the rail had tabs ([C5b notes](../done/C5b-find-in-document.md#notes)); C6a added them.

## Context

- **Design (Paper):** [R05 Find in document](../../design/paper/reader-a.md#r05-find-in-document): the Results tab row of the controls table, the *no matches* state, and the accessibility notes ("Results are links in page order"; the current match isn't shown by colour alone). Build to match; record deviations in the area document.
- [R06 Rail tabs](../../design/paper/reader-a.md#r06-rail-tabs): contextual tabs follow `Pages · Outline`.
- [C5b](../done/C5b-find-in-document.md): `viewer.find` (FindController), the worker's `find` call, `SearchText` and its folding.
- [C6a](../done/C6a-thumbnails-outline-page-labels.md): `railTabs` in `apps/web/src/rail.tsx`, page labels, `currentOutlineItem`.

## Scope

**In scope**

- The worker returns a short snippet with each match, cut from the page's own text around it.
- `viewer.find.results()` (every match found so far, in page order, with its page and snippet) and `viewer.find.goTo(ordinal)`.
- A `FindResults` component in `@laminapdf/ui`: groups headed "Page {label} · {section}", one link per match with the match in weight 600, `aria-current="true"` on the current one; while searching, the matches so far; with none, "No matches for “{query}”." and a ghost button that turns off Match case or Whole words when one is on.
- The web reader adds the tab while find is open and removes it when find closes; a result click goes to the match and closes the rail where it covers the page.

**Out of scope** (and which todo covers it)

- The phone's find layout (a top-bar field, a Previous · Results · Next bar and a non-modal Results sheet): a later phone todo. On a phone the tab shows in the Contents sheet like the others.
- More tabs overflow: the reader has no Attachments tab yet, so three tabs fit.
- Results clicks as history jumps (R07): [C6b](../ongoing/C6b-links-and-history.md).

## Tasks

- [x] `packages/viewer/src/find/page.ts`: `snippetOf(search, folded, range)` → `{ before, match, after }`, white space collapsed, cut at a word near 40 characters each side.
- [x] `protocol.ts` and `worker/find.ts`: `FindPageResult.snippets`, one per match.
- [x] `find/controller.ts`: keep the snippets; `results()` (cached until the next change) and `goTo(ordinal)`.
- [x] `packages/ui/src/reader/find-results.tsx` and its CSS and messages.
- [x] `apps/web/src/rail.tsx` and `app.tsx`: the Results tab while find is open, headings from page labels and the outline.
- [x] Docs: the viewer and UI READMEs, R05's "As built", CLAUDE.md, a changeset.

## Acceptance criteria

- [x] With find open, the rail lists every match of the query in page order under page headings, and clicking one makes it current and scrolls it into view.
- [x] The current match is marked with `aria-current="true"` and follows Next and Previous.
- [x] A query with no matches shows R05's empty state, with the toggle that turns off the active option.
- [x] Closing find removes the tab; the rail shows Pages again.

## Post-merge checks

None.

## Tests

- Unit (Node): `snippetOf` (cuts at words, collapses white space, keeps the match's own case and accents); the controller's `results()` and `goTo`.
- Browser (UI package): `FindResults` groups, the current row, the empty state's toggle, axe.
- e2e (web): open find, see the Results tab, click a result on another page, the page box follows; close find and the tab goes.

## Docs to update

`packages/viewer/README.md` (find's results), `packages/ui/README.md` (`FindResults`), R05's "As built" in `docs/design/paper/reader-a.md`, CLAUDE.md's file map, a changeset for `@laminapdf/viewer` (`@laminapdf/ui` is private).

## Risks and open questions

- A document with tens of thousands of matches: the list renders every row. If the perf harness's common query (950 matches) makes the tab slow to open, virtualise the list.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **Headings.** A group's section is the one in effect at the top of its page (the Outline tab's current section with that page at the top), not the section of each match: matches below a heading that starts lower on the page are listed under the section before. The results carry no position down the page; adding one would let each match take its own section.
- **Snippets.** Cut from the page's own text, so a word joined across a line-end hyphen shows as "exam- ple". Each side keeps up to about 40 characters, cut at a word where the text has spaces and at a whole character where it has none.
- **Not virtualised.** The list renders every row. The perf harness's common query finds 950 matches over 1,000 pages, and the harness doesn't open the tab, so how long that takes is unmeasured. If it proves slow, the list can adopt `OutlineTree`'s `Virtualizer`.
- **No new commands.** The tab has no `find.*` command of its own, so ADR 0024's command list is unchanged.
