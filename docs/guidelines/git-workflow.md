# Git workflow

How changes get from an idea to `main`. The short version: **one worktree, one branch and one PR per piece of work, always branched from an up-to-date `main`.**

**MUST**/**SHOULD** are rules. *Recommendation* marks advice.

## 1. Repository setup: a bare clone with worktrees

We keep the repository as a **bare clone** and check out each branch in its own [worktree](https://git-scm.com/docs/git-worktree). Several pieces of work (yours, or parallel agents') can then be in progress at once without stashing, switching branches or rebuilding over each other's files.

```
lamina.git/                  # bare repository (no working files of its own)
├── main/                    # worktree for main: read-only reference, used to update main
├── docs-code-organization/  # worktree for branch docs/code-organization
└── feat-xref-repair/        # worktree for branch feat/xref-repair
```

One-time setup (if you don't already have it):

```sh
git clone --bare git@github.com:laudebugs/lamina.git lamina.git
cd lamina.git
# A bare clone has no remote-tracking refspec; add one so `git fetch` updates origin/*.
git config remote.origin.fetch '+refs/heads/*:refs/remotes/origin/*'
git fetch origin
git worktree add main main
```

## 2. Starting work

- **MUST:** start every piece of work in a **new worktree off up-to-date `main`**. Never commit in the `main` worktree, and never reuse another task's worktree.

```sh
cd lamina.git                                   # the bare repo
git -C main pull --ff-only origin main          # bring main up to date
git worktree add -b <type>/<desc> <type>-<desc> main
cd <type>-<desc>
```

  For example: `git worktree add -b feat/xref-repair feat-xref-repair main`. Name the folder after the branch, with `/` replaced by `-`.
- **MUST NOT:** use `git stash` to move work between worktrees. The stash (`refs/stash`) is shared by every worktree of the repository (all `refs/` are shared, only pseudo-refs such as `HEAD` are per worktree; [git-worktree](https://git-scm.com/docs/git-worktree#_refs)), so you'd pop someone else's changes, or they'd pop yours. Commit work-in-progress on your branch instead (`git commit -m "wip: …"`, squash later).
- Git won't check out the same branch in two worktrees, and that's a feature. Don't override it with `--force`.
- Dependencies and build output (`node_modules/`, `target/`) are per worktree. Run `pnpm install` / `cargo build` in a new worktree. *Recommendation:* set a shared `CARGO_TARGET_DIR` only if disk space hurts, and accept that builds then lock against each other.

## 3. Branches

- **MUST:** name branches `<type>/<short-description>`, in kebab-case, where `<type>` is a Conventional Commit type: `feat/xref-repair`, `fix/flate-predictor-bounds`, `docs/code-organization`, `spike/uniffi-ios`.
- **MUST:** branches are short-lived. Rebase onto `origin/main` (`git fetch origin && git rebase origin/main`) instead of merging `main` into your branch.
- **MUST NOT:** push directly to `main`. Every change goes through a pull request.

## 4. Commits

- **MUST:** follow [Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): summary`.
  - Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`, `spike`.
  - Scope: the crate, package, app or doc area, without the `lamina-`/`@lamina/` prefix: `feat(core): …`, `fix(wasm): …`, `docs(guidelines): …`.
  - Summary: imperative mood, lowercase, no trailing period, about 72 characters at most.
  - Breaking changes: `feat(core)!: …` and/or a `BREAKING CHANGE:` footer.
- **SHOULD:** each commit builds and passes tests. The body explains *why*, and links issues (`Refs #42`, `Fixes #42`).
- Commits made with an AI agent end with the attribution trailer the agent is configured with (e.g. `Co-Authored-By: …`).

## 5. Pull requests

- **MUST:** one PR per round of work. A PR does one thing (a feature, a fix, a doc set). If you find an unrelated problem, open an issue or a separate PR from a new worktree.
- **SHOULD:** keep PRs reviewable: as a rule of thumb, under ~400 changed lines of code, excluding fixtures, lockfiles and generated files. Split large work into a stack of PRs, each mergeable on its own.
- **MUST:** the PR title is a Conventional Commit line (it becomes the squash-merge commit). The description has a **Summary**, a list of **what changed**, **sources/decisions** it relies on (research chapter, ADR, spec section) and a **Test plan**: the commands you ran and what you checked by hand.
- **MUST:** update docs in the same PR. That covers the folder `README.md` index for new docs, `CLAUDE.md` if commands or layout change, and an ADR if the PR makes a significant decision.
- **MUST:** CI is green before merge: format, lint, typecheck, tests and the portability/`unsafe` checks from [`rust.md`](rust.md) and [`typescript.md`](typescript.md).

## 6. Review

- **MUST:** at least one review before merge (a human, or a human-directed review agent while the project is solo). The author answers or resolves every comment. Fix findings with new commits during review so the reviewer can see what changed, then squash on merge.
- **SHOULD:** reviewers check in this order: correctness and safety (panics, unbounded allocation, `unsafe`), API surface (what became public), boundaries (does core now depend on an edge?), tests (does a test fail without the change?), then docs and style. Anything a tool can enforce is left to the tool.
- **SHOULD:** a regression test for a review finding should fail on the pre-fix code (the spike's `regressions.test.ts` practice).
- Merge with **squash merge** by default. Use a rebase merge when every commit in the PR is meaningful on its own.

## 7. After merge: clean up

```sh
cd lamina.git
git worktree remove <type>-<desc>        # refuses if the worktree has uncommitted changes
git branch -d <type>/<desc>              # use -D if it was squash-merged (Git can't see the merge)
git push origin --delete <type>/<desc>   # unless GitHub deleted it on merge
git worktree prune                       # clears entries for folders deleted by hand
git -C main pull --ff-only origin main
```

- **MUST:** remove the worktree once its PR is merged or abandoned. `git worktree list` should show only live work.

## References

- [git-worktree documentation](https://git-scm.com/docs/git-worktree) (`add -b`, `remove`, `prune`, shared vs per-worktree refs)
- [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)
- GitHub Docs: [About pull request merges](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/about-pull-request-merges)
