# W1e — The progress snapshot: `cargo xtask progress` writes the home page's roadmap data

| Field | Value |
| --- | --- |
| ID | `W1e` |
| Status | `ready` |
| Plan | [Website §4 The home page, for now](../../plans/website.md#the-home-page-for-now-the-roadmap) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | [W1b](../ongoing/W1b-site-scaffold-and-deploys.md) |
| Blocks | [W1c](./W1c-home-page.md) |
| ADRs | — |
| Branch | `feat/w1e-progress-snapshot` |
| PR title | `feat(xtask): cargo xtask progress, the snapshot behind the website's roadmap` |
| Size | M (200–400 lines of Rust and tests, plus the first snapshot) |
| PR | — |

## Goal

`cargo xtask progress` writes `apps/site/src/data/progress.json`, the one file the home page's roadmap and dashboard read ([W1c](./W1c-home-page.md)). It counts the todo board with the board's own parser, so the dashboard and `docs/todos/README.md` never disagree. Anyone can refresh it from time to time with one command and an exempt PR.

## Context

- [Website plan §4](../../plans/website.md#the-home-page-for-now-the-roadmap): what the dashboard shows and what it leaves out (people's names, links into the private repository).
- `tools/xtask/src/todos.rs`: the board's parser and generator (states, rounds in board order, dependencies, the "Can start now" list). The snapshot reuses it rather than reading the todo files a second way ([maintainability](../../guidelines/maintainability.md): one home per concept).
- [Roadmap § Milestones](../../plans/roadmap.md#milestones): each milestone's name, rounds (`B1–B7`, `B9, C1–C7, U1, U2, U3a, U4a`) and what it ships.
- Squash-merged commits on `main` end in `(#<n>)`, which counts merged pull requests.
- [Git workflow §2](../../guidelines/git-workflow.md#pick-work-from-the-todo-board): the exempt PRs, which gain the snapshot refresh.

## Scope

**In scope**

- `cargo xtask progress [--offline]` (`tools/xtask/src/progress.rs`), writing pretty JSON with a trailing newline, in a stable order, with a `version` field and the snapshot's date:
  - from the todo files, through `todos.rs`: done, ongoing and ready counts per round in the board's order; the "Can start now" todos and the ongoing ones, by ID and title;
  - from the roadmap's Milestones table: each milestone's name, what it ships, and its state: done when every todo of its rounds is done, under way when any is done or claimed, not started otherwise. A round range (`C1–C7`) or a round (`U1`) matches every todo of those rounds; a todo ID (`U3a`) matches itself;
  - from `git log` on `main`: the number of merged pull requests and the last ten merge subjects, without their PR numbers;
  - from the open pull requests (`gh pr list`): the todos they claim, matched by the todo's **Branch** field, shown by the todo's title. `--offline`, or `gh` failing, keeps the in-progress list of the last snapshot and says so in a warning and in the file, since claims live only on PR branches and `ongoing/` on `main` is always empty.
- A TypeScript type for the file in `apps/site` and a test that the committed file matches it.
- The first snapshot, committed.
- [Git workflow §2](../../guidelines/git-workflow.md#pick-work-from-the-todo-board) and `CLAUDE.md`: a snapshot refresh is an exempt PR; `CLAUDE.md`'s commands list `cargo xtask progress`.

**Out of scope**

- The page → [W1c](./W1c-home-page.md).
- A scheduled refresh workflow (optional; it needs the owner to commit a workflow file).

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] Expose what the snapshot needs from `todos.rs` (counts per round, the startable and ongoing todos) without a second parser.
- [ ] The milestone table reader and the state rule.
- [ ] The `git log` and `gh` readers behind functions the tests replace with fixtures.
- [ ] Write the file; the TypeScript type and its test; the first snapshot.
- [ ] Git workflow §2 and `CLAUDE.md`.

## Acceptance criteria

- [ ] For one set of fixture todo files, the snapshot's counts per round and its "Can start now" list equal the board's.
- [ ] `--offline` writes a valid file with neither the network nor `gh`.
- [ ] The milestone states for the fixture board match a hand-checked table, including a range (`C1–C7`) and a single todo (`U3a`).
- [ ] The file holds no author, no session name and no PR number.

## Tests

Unit tests over fixture todo files and a fixture milestone table; merge-subject parsing; the TypeScript shape test.

## Docs to update

`CLAUDE.md` (commands), [git workflow §2](../../guidelines/git-workflow.md#pick-work-from-the-todo-board), the [website plan §8](../../plans/website.md#8-to-dos).

## Risks and open questions

- **Milestone rows mix rounds and todos.** Parse both forms, and fail on a token that matches no todo, so a renamed round can't silently drop out of its milestone.

## Definition of done

- [ ] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the guideline and `CLAUDE.md` text
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
