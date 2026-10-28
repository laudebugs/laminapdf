# Contributing

Start here. This page is the checklist. The linked guides have the rules and the reasons behind them.

## Before you start

1. Read [`CLAUDE.md`](../../CLAUDE.md) for the working agreements and commands.
2. For PDF features, read the relevant chapter in [`docs/research/`](../research/) and the ISO 32000 section first (*spec first*).
3. Check [`docs/decisions/`](../decisions/) for ADRs that constrain the change.

## The workflow in brief

1. **Create a new git worktree off up-to-date `main`** with a branch named `<type>/<short-description>`: `git worktree add -b feat/xref-repair feat-xref-repair main`. See [git-workflow.md](git-workflow.md).
2. Make one focused change, following [code-organization.md](code-organization.md) and the language guide ([rust.md](rust.md), [typescript.md](typescript.md)).
3. Commit with [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): summary`. Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`, `spike`.
4. Push and open a pull request against `main`, with a summary, sources and a test plan. Don't push directly to `main`.
5. Get a review, address every comment, and merge once checks pass.
6. Remove the worktree and branch.

## Documentation

- Anything worth remembering goes in `docs/`. See [`docs/README.md`](../README.md) for where.
- Significant technical decisions get an ADR in `docs/decisions/`.
- Add every new doc to its folder's `README.md` index.
- Cite primary sources, and label recommendations and opinions as such.

## Dependencies and licences

Check the licence of every new dependency, fixture PDF and font before adding it. No AGPL/GPL/LGPL code goes into shipped artefacts. See [research ch. 05 §8.4](../research/pdf/05-creating-pdfs-from-code.md#84-licensing-traps-to-avoid).
