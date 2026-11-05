# W1e — The progress snapshot: `cargo xtask progress` writes the home page's roadmap data

| Field | Value |
| --- | --- |
| ID | `W1e` |
| Status | `done` |
| Plan | [Website §4 The home page, for now](../../plans/website.md#the-home-page-for-now-the-roadmap) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | [W1b](./W1b-site-scaffold-and-deploys.md) |
| Blocks | [W1c](./W1c-home-page.md) |
| ADRs | — |
| Branch | `feat/w1e-progress-snapshot` |
| PR title | `feat(xtask): cargo xtask progress, the snapshot behind the website's roadmap` |
| Size | M (200–400 lines of Rust and tests, plus the first snapshot) |
| PR | [#248](https://github.com/laudebugs/laminapdf/pull/248) |

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

- [x] Claim: the first commit moves this file to `ongoing/`.
- [x] Expose what the snapshot needs from `todos.rs` (counts per round, the startable and ongoing todos) without a second parser.
- [x] The milestone table reader and the state rule.
- [x] The `git log` and `gh` readers behind functions the tests replace with fixtures.
- [x] Write the file; the TypeScript type and its test; the first snapshot.
- [x] Git workflow §2 and `CLAUDE.md`.

## Acceptance criteria

- [x] For one set of fixture todo files, the snapshot's counts per round and its "Can start now" list equal the board's.
- [x] `--offline` writes a valid file with neither the network nor `gh`.
- [x] The milestone states for the fixture board match a hand-checked table, including a range (`C1–C7`) and a single todo (`U3a`).
- [x] The file holds no author, no session name and no PR number.

## Tests

Unit tests over fixture todo files and a fixture milestone table; merge-subject parsing; the TypeScript shape test.

## Docs to update

`CLAUDE.md` (commands), [git workflow §2](../../guidelines/git-workflow.md#pick-work-from-the-todo-board), the [website plan §8](../../plans/website.md#8-to-dos).

## Risks and open questions

- **Milestone rows mix rounds and todos.** Parse both forms, and fail on a token that matches no todo, so a renamed round can't silently drop out of its milestone.

## Notes from the work

- **Claims come from the files a pull request moves, not its branch.** A pull request claims a todo when it adds the todo's file under `ongoing/` or `done/`, which is what claiming means on the board. The **Branch** field turned out to be unreliable: on 2026-11-05 neither U6e's pull request (`claude/working-session-9g3nst`) nor L3e's (`feat/l3e-extract-bindings`) used the branch its todo names. The open pull requests are read with `gh api` (REST), one call for the list and one per pull request for its files.
- **"Rounds" in the file are the board's sections** (`A`, `B`, `L` …), each with its heading, because the dashboard draws one bar per section. A ready todo that an open pull request claims counts as in progress, so a section's `todos` is `done + inProgress + ready`.
- **The roadmap's Rounds cells** mix four forms: a section letter (`A`), a range (`B1–B7`), a round (`U1`) and a todo (`U3a`). A token that matches no todo fails the command.
- **It reads the board and the roadmap from `main`** (`origin/main` when there is one, with `git show`), not from the checkout, so a branch's own todo moves never reach the snapshot, and "Next up" leaves out the todos already in progress.
- **`gh` moved to `util.rs`**, shared with `release-guard`, and runs in the repository's folder so `{owner}/{repo}` resolves.

## Definition of done

- [x] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the guideline and `CLAUDE.md` text
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
