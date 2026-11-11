# W3f — Coding agents find the docs

| Field | Value |
| --- | --- |
| ID | `W3f` |
| Status | `ongoing` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3b](../done/W3b-public-repository-links.md) |
| ADRs | None |
| Branch | `docs/w3f-agent-discovery` |
| PR title | `docs: a Context7 listing, and llms.txt linked where agents look` |
| Size | S |
| PR | [#448](https://github.com/laudebugs/laminapdf/pull/448) |

## Goal

Coding agents look up a library's current docs through tools such as Context7, or through a site's llms.txt when a person points them at it. L5a built llms.txt, llms-full.txt and a Markdown twin of every docs page, and deferred the Context7 listing until the packages were on the registries, which they now are. Nothing on the website or in the README links llms.txt. This todo lists the docs with Context7, claims the listing, and links the agent docs where people and agents look first.

## Context

- [Discoverability research §3](../../research/discoverability/search-and-answer-engines.md#3-llmstxt-helps-coding-agents-not-search-rankings): what llms.txt does and doesn't do; Context7's add-library page, its `/v2/add/llmstxt` API and `context7.json`.
- [L5a](../done/L5a-agent-docs.md): its out-of-scope note on Context7 and its unticked post-merge check that production serves the files.
- `apps/docs/src/content/docs/start/coding-agents.mdx`: the one page that names the agent docs today.

## Scope

**In scope**

- `context7.json` at the repository root (Context7's format: the project's title, description, the folders to read or exclude, and rules for agents from `packages/sdk/examples/AGENTS.md`), and the listing submitted from `https://docs.laminapdf.com/llms.txt`.
- A link to the docs' llms.txt and the "Using Lamina PDF with a coding agent" page from the website's home page and footer, and from the root README.
- L5a's post-merge check, ticked in its done file with the URLs.
- The owner's decision on Cloudflare's Markdown for Agents ([plan §4](../../plans/discoverability.md#4-decisions-for-the-owner), decision 4), recorded.

**Out of scope**

- An MCP docs server and a `laminapdf docs <symbol>` command: still L5a's deferred items.

## Tasks

- [x] Read Context7's current docs for `context7.json` and the add-library flow; write the file; submit the listing (the owner, if it needs an account) and record its URL. The owner submitted it on 2026-11-11: [context7.com/llmstxt/laminapdf_llms_txt](https://context7.com/llmstxt/laminapdf_llms_txt). An llms.txt listing is claimed by a `context7.json` under the docs' own address, so the docs serve `apps/docs/public/context7.json` (its `url` and `public_key`) at `https://docs.laminapdf.com/context7.json`; the owner finishes the claim once a release deploys it. Context7 also built [`/laudebugs/laminapdf`](https://context7.com/laudebugs/laminapdf) from the repository; the root `context7.json` configures it and holds its claim (`url`, `public_key`), which Context7 reads from `main` once this merges.
- [ ] Website links (the owner approves any change to the home page's words) and the README line.
- [ ] Check `/llms.txt`, `/llms-full.txt` and one twin on production; tick L5a's check.
- [ ] Ask Context7's MCP server a question about Lamina PDF and note the answer in the PR.

## Acceptance criteria

- [ ] Context7 lists Lamina PDF from the docs' llms.txt, claimed by the repository's `context7.json`.
- [ ] The website and the README link the agent docs.

## Post-merge checks

- [ ] Context7 refreshes the listing after the next release.

## Tests

The website's build checks (internal and external links); `cargo xtask docs-links`.

## Docs to update

The root README, the website copy (owner-approved), `docs/todos/done/L5a-agent-docs.md`, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- Context7 may index the repository's Markdown rather than llms.txt; `context7.json`'s folder rules keep it to the docs and the examples.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
