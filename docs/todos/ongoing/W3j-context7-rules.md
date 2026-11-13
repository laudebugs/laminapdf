# W3j — Context7's rules match the agent snippet

| Field | Value |
| --- | --- |
| ID | `W3j` |
| Status | `ongoing` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3f](../done/W3f-agent-discovery.md) |
| ADRs | None |
| Branch | `claude/inspiring-lamport-q8469n` (this session's branch) |
| PR title | `docs: bring Context7's rules in line with the agent snippet` |
| Size | S |
| PR | — |

## Goal

An agent that asks Context7 about Lamina PDF reads the rules in the root `context7.json`. W3f wrote them from the snippet that ships with the packages (`packages/sdk/examples/AGENTS.md`), and the two have drifted since. The rules leave out building without code, the document schema the packages ship, and the flow templates' errors. Neither tells an agent how to extract text or which `laminapdf` commands and MCP tools exist. This todo brings the rules and the snippet back in step and adds what both lack. It also links a root `AGENTS.md` to `CLAUDE.md`, so agents that read `AGENTS.md` find the repository's own instructions.

## Context

- [W3f](../done/W3f-agent-discovery.md): wrote `context7.json` and its rules from the snippet.
- `packages/sdk/examples/AGENTS.md`: the snippet. `apps/docs/scripts/package-docs.mjs` copies it into the packages' `docs/AGENTS.md`, and the agent eval (`tools/agent-eval/`) writes it into its test projects.
- [Extract text and Markdown](../../../apps/docs/src/content/docs/guides/extract-text.mdx), [Inspect and check PDFs](../../../apps/docs/src/content/docs/cli/inspect.mdx) and [the MCP page](../../../apps/docs/src/content/docs/cli/mcp.mdx): the facts the new rules state.

## Scope

**In scope**

- A root `AGENTS.md` that links to `CLAUDE.md`, and its line in `CLAUDE.md`'s layout.
- `context7.json`'s `rules`: every point of the snippet, shortened, plus extraction and the command line.
- The snippet: a point on extraction and one on the command line and MCP tools, so it says what the rules say.

**Out of scope**

- A check that the rules and the snippet agree. They word the same points differently (Context7's rules are short), so a check would need a list of points both are held to. Left for a follow-up if they drift again.
- `apps/docs/public/context7.json`, which holds only the llms.txt listing's claim.
- The generated copy in `crates/laminapdf/docs/`, which the release rewrites from the snippet.

## Tasks

- [x] Link `AGENTS.md` to `CLAUDE.md`; add it to the layout in `CLAUDE.md`.
- [ ] Rewrite `context7.json`'s rules from the snippet; add extraction and the command line.
- [ ] Add the two points to the snippet.

## Acceptance criteria

- [ ] Each point of the snippet has a rule in `context7.json`, and each rule a point in the snippet.
- [ ] Every API name, option, command and tool a rule names exists, as the docs pages above show it.
- [ ] `cargo xtask check docs` passes.

## Post-merge checks

- [ ] Context7 reads the new rules from `main` at its next refresh of [`/laudebugs/laminapdf`](https://context7.com/laudebugs/laminapdf).

## Tests

`cargo xtask check docs`; `pnpm --filter @laminapdf/docs test` (the package docs script copies the snippet).

## Docs to update

The [discoverability plan](../../plans/discoverability.md) (its table and to-dos) and `CLAUDE.md`'s layout.

## Risks and open questions

- Context7 may cap a rule's length; the rules stay a sentence or two each.

## Definition of done

- [x] Claimed: the work PR moved this file to `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
