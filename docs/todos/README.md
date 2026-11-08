# Todo board

Each todo is one pull request of work, and its folder is its state. To pick work, run `cargo xtask todos`: it lists the todos you can start now and writes the whole board to `target/todos/board.md`.

| Folder | Meaning |
| --- | --- |
| [`ready/`](ready/) | Specified and waiting. Start one only when every todo in its **Depends on** is in `done/`. |
| [`ongoing/`](ongoing/) | Claimed: someone is working on it. Moved here by the first commit of its work PR, so `main` shows a todo here only once that PR merges; open PRs hold the current claims (`gh pr list`). |
| [`done/`](done/) | Merged. Moved here by the work PR before it merges. |

New todos start from [`TEMPLATE.md`](TEMPLATE.md). The workflow is in [git workflow §2](../guidelines/git-workflow.md#pick-work-from-the-todo-board); plans link to their todos.

## The board is built on demand

`cargo xtask todos` builds the board from the todo files and prints two lists. "Can start now" holds the ready todos whose dependencies are all done; an open PR may already have claimed one, so check `gh pr list`. "Ongoing" holds the todos in `ongoing/`. The whole board goes to `target/todos/board.md`, outside git: the counts, the same two lists, and one table per plan with each todo's state, what it depends on and what it blocks. Without a checkout, the [website's home page](https://laminapdf.com) shows the counts, the todos in progress and the ones next up, from `cargo xtask progress`.

Only what people write is committed. The board's counts and lists depend on many todos at once, so a committed board would change in every PR that moves a todo, and each merge to `main` would make the other open PRs conflict on it. For the same reason, todo files have no **Blocks** row: **Depends on** is the one record of the graph, and the board shows it in reverse.

## The checks

`cargo xtask todos` also points each **Depends on** link at the folder its todo is in now. `cargo xtask check` runs `cargo xtask todos --check`, which changes nothing and fails on four kinds of problem:

- a graph that doesn't hold together: a **Status** that doesn't match the folder, an unknown dependency, a cycle, or a done todo that depends on an unfinished one;
- a **Depends on** link that points at an old folder;
- a **Blocks** row left in a todo file;
- a link from this page to a todo file, which is how a merge would bring a stale board back.

`cargo xtask docs-links` fails on any broken link.

## IDs

`A`/`P`/`X` [architecture](../plans/architecture.md) (foundations, publishing, cross-cutting), `B` [library](../plans/pdf-library.md), `L` [shareable libraries](../plans/shareable-libraries.md) (contracts, worker, extraction, CLI, agent docs, mobile), `C` [web reader](../plans/web-app.md), `W` [website](../plans/website.md), `D` [desktop reader](../plans/desktop-app.md), `U` [UI/UX](../plans/ui-ux.md), `E` [editor](../plans/full-editor.md), `H` hosted service ([full-editor](../plans/full-editor.md)), `Q` [docs and code quality](../plans/quality.md), `T` [trust and proof](../plans/trust-and-proof.md). A letter suffix (`B2a`, `B2b`) splits a round into PRs.
