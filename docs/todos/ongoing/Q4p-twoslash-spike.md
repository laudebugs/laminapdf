# Q4p — `twoslash` spike: hover types in the docs, at what cost?

| Field | Value |
| --- | --- |
| ID | `Q4p` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4i](../ready/Q4i-code-colours-by-class.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the headers, no third-party loads) |
| Branch | `spike/q4p-twoslash` |
| PR title | `spike(docs): measure twoslash hover types on one docs page` |
| Size | S (throwaway spike code outside the workspaces; README with measurements) |
| PR | [#453](https://github.com/laudebugs/laminapdf/pull/453) |

## Goal

The docs plan left Twoslash out on a cost nobody measured. The research guessed that its hover types would add script to every code block and probably inline styles, and it never ran the Expressive Code plugin. This spike puts Twoslash on one docs page and measures what it costs. The plan then keeps the decision or turns it into a todo.

## Context

- [Docs research: what to leave out](../../research/docs-site/developer-docs.md) and its notes: [frameworks Q4](../../research/docs-site/developer-docs/docs_frameworks.md#q4-interactivity-live-code-twoslash-synced-tabs-islands), where the plugin's status is marked unverified, and [exemplary and interactive docs](../../research/docs-site/developer-docs/exemplary_and_interactive_docs.md) (what Twoslash shows, and why it suits a typed SDK).
- [Docs redesign §5](../../plans/docs-redesign.md#5-out-of-scope), which lists Twoslash as out of scope.
- Q4i leaves the docs' CSP with no `'unsafe-inline'` in `style-src`, so this spike measures against the CSP the site will have.
- `apps/docs/public/_headers` (the CSP), `apps/docs/astro.config.mjs` (Starlight, whose code blocks are Expressive Code), and `apps/docs/src/components/Snippet.astro` (code from the tested example files).
- Why it might be worth it: `@laminapdf/sdk` is a typed API, and hover types would show a reader what `Builder`, `Document` and the option objects hold without leaving the page.

## Scope

**In scope**

- `spikes/docs-twoslash/`: a copy of the docs site, or a branch build of it, with `expressive-code-twoslash` (or the maintained alternative, if that plugin is abandoned) on one page: "Your first PDF" in TypeScript.
- The plugin's licence and its dependencies' licences checked against [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) before it is installed.
- Measurements on that page, before and after:
  - JavaScript and CSS the page loads (raw and gzip), and whether any of it comes from another origin;
  - `style=` attributes and inline `<style>` or `<script>` blocks in the built HTML, and violations of the site's CSP in Chromium, Firefox and WebKit;
  - build time of the whole docs site;
  - what a reader gets: hovers by mouse and by keyboard focus, on touch, in both themes, and to a screen reader.
- Whether the types come from the workspace's `@laminapdf/sdk` build, so a renamed or removed API fails the docs build.
- A README with the table, screenshots, and a recommendation: adopt (with a todo drafted), adopt on chosen pages only, or keep it out.

**Out of scope** (and which todo covers it)

- Adding Twoslash to the docs site: a new todo, if the README recommends it.
- Rust code blocks: Twoslash is TypeScript-only.

## Tasks

- [ ] Claim via the work PR (see Definition of done) from a fresh worktree off `main`, after Q4i has merged.
- [ ] Check the plugin's maintenance and licence; pick it or the alternative, and say why in the README.
- [ ] Build the page with and without Twoslash, and record each measurement in Scope.
- [ ] Try the hovers as a reader would, by mouse, keyboard and touch, in Chromium, Firefox and WebKit, and record what works.
- [ ] Rename a type in the SDK and check whether the docs build fails.
- [ ] Write the recommendation, and update the plan's §5 with the result (and its §4 if a todo follows).

## Acceptance criteria

- [ ] The README answers, with numbers, how much script and style the page gains, whether the page passes the CSP Q4i sets, and whether a stale type fails the build.
- [ ] The README records what keyboard, touch and screen-reader users get from the hovers.
- [ ] The plan's §5 states the measured result instead of the research's guess.

## Post-merge checks

None.

## Tests

None beyond the spike's own measurements; nothing ships.

## Docs to update

[Docs redesign §5](../../plans/docs-redesign.md#5-out-of-scope), and §4 if the recommendation is to adopt; the frameworks note's line that marks the plugin unverified.

## Risks and open questions

- The plugin may fit only an older Expressive Code than Starlight's; the README then says which versions and what an upgrade would take.
- Hovers that need a mouse leave keyboard and touch readers with nothing; a fallback (the type shown inline with `// ^?`) may be the only part worth keeping.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
