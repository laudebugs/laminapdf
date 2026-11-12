# Q4p — `twoslash` spike: hover types in the docs, at what cost?

| Field | Value |
| --- | --- |
| ID | `Q4p` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
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

- [x] Claim via the work PR (see Definition of done) from a fresh worktree off `main`, after Q4i has merged.
- [x] Check the plugin's maintenance and licence; pick it or the alternative, and say why in the README.
- [x] Build the page with and without Twoslash, and record each measurement in Scope.
- [x] Try the hovers as a reader would, by mouse, keyboard and touch, in Chromium, Firefox and WebKit, and record what works.
- [x] Rename a type in the SDK and check whether the docs build fails.
- [x] Write the recommendation, and update the plan's §5 with the result (and its §4 if a todo follows).

## Acceptance criteria

- [x] The README answers, with numbers, how much script and style the page gains, whether the page passes the CSP Q4i sets, and whether a stale type fails the build.
- [x] The README records what keyboard, touch and screen-reader users get from the hovers.
- [x] The plan's §5 states the measured result instead of the research's guess.

## Post-merge checks

None.

## Tests

None beyond the spike's own measurements; nothing ships.

## Docs to update

[Docs redesign §5](../../plans/docs-redesign.md#5-out-of-scope), and §4 if the recommendation is to adopt; the frameworks note's line that marks the plugin unverified.

## Risks and open questions

- The plugin may fit only an older Expressive Code than Starlight's; the README then says which versions and what an upgrade would take.
- Hovers that need a mouse leave keyboard and touch readers with nothing; a fallback (the type shown inline with `// ^?`) may be the only part worth keeping.

## As built

- **Q4i first?** Q4i hadn't started when the owner asked for this spike, so its dependency on [Q4i](../ready/Q4i-code-colours-by-class.md) was dropped and the spike measures against both CSPs: the current one, and the same policy without `'unsafe-inline'` in `style-src`, which is what Q4i will set.
- **The plugin:** `expressive-code-twoslash` 0.6.1, the only Twoslash plugin for Expressive Code. It supports Expressive Code 0.41 and TypeScript 5 only, while the site runs 0.44.3 and 6.0.3. npm refuses it and pnpm installs it with peer warnings. It works once Expressive Code's options move to `ec.config.mjs`, which Starlight's `<Code>` needs for plugins.
- **Where the code lives:** `spikes/docs-twoslash/` holds a patch for `apps/docs` (a `twoslash` prop through `Example`, `ExampleCode` and `Snippet`, set on "Your first PDF"; the plugin behind `LAMINA_PDF_TWOSLASH=1`), `measure.mjs`, the two JSON reports and two screenshots. The site itself is unchanged.
- **Result:** keep it out. It adds 24.5 KB of script and 15.5 KB of CSS to every page. On the one page with hovers it adds 953 `style=` attributes, nearly doubles the HTML, and adds 1.7 s (10%) to the build. Hovers open by mouse only (touch too in Firefox and WebKit), and screen readers get nothing. A renamed type fails the build with it, but `cargo xtask check ts` already catches that, since the docs show only tested example files.
- **Screen readers:** judged from the popup's attributes (`role="tooltip"`, `aria-hidden="true"`, no focusable token), not tested with a screen reader.
- **Plan:** §5 states the measured result, and the frameworks note marks the plugin as measured. No todo follows.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
